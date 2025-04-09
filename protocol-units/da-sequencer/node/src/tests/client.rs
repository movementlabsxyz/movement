use crate::{
	batch::FullNodeTxs,
	run,
	server::run_server,
	tests::{
		generate_signing_key, make_test_whitelist,
		mock::{mock_wait_and_get_next_block, mock_write_new_batch, CelestiaMock, StorageMock},
	},
};
use ed25519_dalek::{Signature, Signer};
use futures::StreamExt;
use movement_da_sequencer_client::{
	serialize_full_node_batch, DaSequencerClient, GrpcDaSequencerClient,
};
use movement_da_sequencer_config::DaSequencerConfig;
use movement_da_sequencer_proto::{BatchWriteRequest, StreamReadFromHeightRequest};
use movement_signer::cryptography::ed25519::Signature as SigningSignature;
use movement_types::transaction::Transaction;
use std::net::SocketAddr;
use tokio::sync::mpsc;
use url::Url;

#[tokio::test]
async fn test_should_write_batch() {
	let (request_tx, request_rx) = mpsc::channel(100);

	let config = DaSequencerConfig::default();
	let signing_key = generate_signing_key();
	let verifying_key = signing_key.verifying_key();
	let whitelist = make_test_whitelist(vec![verifying_key.clone()]);

	// Start gprc server
	// Start gprc server. Define a different address for each test.
	let grpc_address = "0.0.0.0:30700"
		.parse::<SocketAddr>()
		.expect("Bad da sequencer listener address.");
	let grpc_jh =
		tokio::spawn(async move { run_server(grpc_address, request_tx, whitelist).await });

	//start main loop
	let storage_mock = StorageMock::new();
	let celestia_mock = CelestiaMock::new();
	let loop_jh = tokio::spawn(run(config, request_rx, storage_mock, celestia_mock));

	//need to wait the server is started before connecting
	let _ = tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

	let connection_url = Url::parse(&format!("http://127.0.0.1:{}", grpc_address.port())).unwrap();
	let mut client = GrpcDaSequencerClient::try_connect(&connection_url.clone())
		.await
		.expect("gRPC client connection failed.");

	mock_write_new_batch(&mut client, &signing_key, verifying_key).await;

	//verify the block produced contains the batch.
	//Register to block stream
	let mut client = GrpcDaSequencerClient::try_connect(&connection_url)
		.await
		.expect("gRPC client connection failed.");

	let mut block_stream = client
		.stream_read_from_height(StreamReadFromHeightRequest { height: 0 })
		.await
		.expect("Failed to register to block stream");

	//wait at least one block production
	let _ = tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
	mock_wait_and_get_next_block(&mut block_stream, 1).await;

	grpc_jh.abort();
	loop_jh.abort();
}

#[tokio::test]
async fn test_write_batch_gprc_main_loop_failed_validate_batch() {
	let (request_tx, request_rx) = mpsc::channel(100);

	let config = DaSequencerConfig::default();
	let signing_key = generate_signing_key();
	let verifying_key = signing_key.verifying_key();
	let whitelist = make_test_whitelist(vec![verifying_key.clone()]);

	// Start gprc server. Define a different address for each test.
	let grpc_address = "0.0.0.0:30701"
		.parse::<SocketAddr>()
		.expect("Bad da sequencer listener address.");

	let grpc_jh =
		tokio::spawn(async move { run_server(grpc_address, request_tx, whitelist).await });

	//start main loop
	let storage_mock = StorageMock::new();
	let celestia_mock = CelestiaMock::new();
	let loop_jh = tokio::spawn(run(config, request_rx, storage_mock, celestia_mock));

	//need to wait the server is started before connecting
	let _ = tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

	let tx = Transaction::test_only_new(b"test data".to_vec(), 1, 123);

	let txs = FullNodeTxs::new(vec![tx]);

	let batch_bytes = bcs::to_bytes(&txs).expect("Serialization failed");
	//define a dummy signature for the batch
	let signature = Signature::from_bytes(&[0; 64]);
	let signature = SigningSignature::try_from(&signature.to_bytes()[..]).unwrap();

	// Serialize full node batch into raw bytes
	let serialized =
		serialize_full_node_batch(verifying_key, signature.clone(), batch_bytes.clone());

	//send the bacth using the grpc client
	let connection_url = Url::parse(&format!("http://127.0.0.1:{}", grpc_address.port())).unwrap();
	let mut client = GrpcDaSequencerClient::try_connect(&connection_url)
		.await
		.expect("gRPC client connection failed.");

	let request = BatchWriteRequest { data: serialized };
	let res = client.batch_write(request).await.expect("Failed to send the batch.");
	tracing::info!("{res:?}",);
	//return false because of the signature.
	assert!(!res.answer);

	//TODO verify no block has been produced.
	//Register to block stream
	let mut client = GrpcDaSequencerClient::try_connect(&connection_url)
		.await
		.expect("gRPC client connection failed.");

	let mut block_stream = client
		.stream_read_from_height(StreamReadFromHeightRequest { height: 0 })
		.await
		.expect("Failed to register to block stream");

	//wait at least one block production
	let _ = tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
	//test there's no block. Batch has been rejected.
	if let Ok(Some(Ok(block))) =
		tokio::time::timeout(std::time::Duration::from_secs(1), block_stream.next()).await
	{
		panic!("Error get a genesis block at height 0 {block:?}");
	}

	grpc_jh.abort();
	loop_jh.abort();
}

#[tokio::test]
async fn test_produce_block_and_stream() {
	let (request_tx, request_rx) = mpsc::channel(100);

	let mut config = DaSequencerConfig::default();
	//update config to generate faster heartbeat.
	config.stream_heartbeat_interval_sec = 1;
	let signing_key = generate_signing_key();
	let verifying_key = signing_key.verifying_key();
	let whitelist = make_test_whitelist(vec![verifying_key.clone()]);

	// Start gprc server
	// Start gprc server. Define a different address for each test.
	let grpc_address = "0.0.0.0:30702"
		.parse::<SocketAddr>()
		.expect("Bad da sequencer listener address.");
	let grpc_jh =
		tokio::spawn(async move { run_server(grpc_address, request_tx, whitelist).await });

	//start main loop
	let storage_mock = StorageMock::new();
	let celestia_mock = CelestiaMock::new();
	let loop_jh = tokio::spawn(run(config, request_rx, storage_mock, celestia_mock));

	//need to wait the server is started before connecting
	let _ = tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

	//Register to block stream
	let connection_url = Url::parse(&format!("http://127.0.0.1:{}", grpc_address.port())).unwrap();
	let mut client = GrpcDaSequencerClient::try_connect(&connection_url)
		.await
		.expect("gRPC client connection failed.");

	let mut block_stream = client
		.stream_read_from_height(StreamReadFromHeightRequest { height: 0 })
		.await
		.expect("Failed to register to block stream");

	// Wait block production.
	let _ = tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
	//test there's no block. Genesis block can't be retrieved
	if let Ok(Some(Ok(block))) =
		tokio::time::timeout(std::time::Duration::from_secs(1), block_stream.next()).await
	{
		panic!("Error get a genesis block at height 0 {block:?}");
	}

	mock_write_new_batch(&mut client, &signing_key, verifying_key).await;

	// Wait for the block produced and streamed.
	mock_wait_and_get_next_block(&mut block_stream, 1).await;

	//write 2 batch to produce 2 blocks.
	mock_write_new_batch(&mut client, &signing_key, verifying_key).await;
	let _ = tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
	mock_write_new_batch(&mut client, &signing_key, verifying_key).await;
	let _ = tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

	//get the 2 blocks
	mock_wait_and_get_next_block(&mut block_stream, 2).await;
	mock_wait_and_get_next_block(&mut block_stream, 3).await;

	//create a new client and see if it steam all blocks.
	let mut client2 = GrpcDaSequencerClient::try_connect(&connection_url)
		.await
		.expect("gRPC client connection failed.");
	let mut block_stream2 = client2
		.stream_read_from_height(StreamReadFromHeightRequest { height: 2 })
		.await
		.expect("Failed to register to block stream");
	mock_wait_and_get_next_block(&mut block_stream2, 2).await;
	mock_wait_and_get_next_block(&mut block_stream2, 3).await;

	//write a batch and see if both clients stream the new block.
	mock_write_new_batch(&mut client, &signing_key, verifying_key).await;
	let _ = tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
	mock_wait_and_get_next_block(&mut block_stream2, 4).await;

	//wait at least one block production
	let _ = tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;

	// Wait enought to see if heartbeat are filtered.
	if let Ok(Some(Ok(block))) =
		tokio::time::timeout(std::time::Duration::from_secs(1), block_stream2.next()).await
	{
		panic!("Error get block without batch: {block:?}");
	}

	//the other stream should get the block 4.
	mock_wait_and_get_next_block(&mut block_stream, 4).await;

	grpc_jh.abort();
	loop_jh.abort();
}

/// Submit a batch using the same key to sign and verifying using the white list.
#[tokio::test]
async fn test_grpc_client_should_write_one_batch_with_a_correct_whitelist() {
	let config = DaSequencerConfig::default();

	let signing_key = generate_signing_key();
	let verifying_key = signing_key.verifying_key();
	let whitelist = make_test_whitelist(vec![verifying_key.clone()]);
	let (request_tx, request_rx) = mpsc::channel(100);

	// Start gprc server. Define a different address for each test.
	let grpc_address = "0.0.0.0:30703"
		.parse::<SocketAddr>()
		.expect("Bad da sequencer listener address.");
	let grpc_task = tokio::spawn(run_server(grpc_address, request_tx, whitelist));
	let main_loop = tokio::spawn(async move {
		let storage = StorageMock::new();
		let da = CelestiaMock::new();
		run(config, request_rx, storage, da).await.unwrap();
	});

	let _ = tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

	let connection_url = Url::parse(&format!("http://{}", grpc_address)).unwrap();
	let mut client = GrpcDaSequencerClient::try_connect(&connection_url)
		.await
		.expect("Failed to connect");

	let tx = Transaction::test_only_new(b"abc".to_vec(), 1, 123);
	let txs = FullNodeTxs::new(vec![tx]);
	let batch_bytes = bcs::to_bytes(&txs).unwrap();
	let signature = signing_key.sign(&batch_bytes);
	let signature = SigningSignature::try_from(&signature.to_bytes()[..]).unwrap();
	let data = serialize_full_node_batch(verifying_key, signature, batch_bytes);

	let res = client.batch_write(BatchWriteRequest { data }).await;
	let response = res.expect("batch_write failed");
	assert!(response.answer);

	grpc_task.abort();
	let _ = grpc_task.await;

	main_loop.abort();
	let _ = main_loop.await;
}

/// Submit a batch with an empty the white list.
/// The batch is rejected because the submitter public key is not part of the white list.
#[tokio::test]
async fn test_grpc_client_should_write_one_batch_with_an_empty_whitelist() {
	let (request_tx, request_rx) = mpsc::channel(100);

	let config = DaSequencerConfig::default();
	let signing_key = generate_signing_key();
	let verifying_key = signing_key.verifying_key();

	let whitelist = make_test_whitelist(vec![]);

	// Start gprc server. Define a different address for each test.
	let grpc_address = "0.0.0.0:30704"
		.parse::<SocketAddr>()
		.expect("Bad da sequencer listener address.");
	let _grpc_jh =
		tokio::spawn(async move { run_server(grpc_address, request_tx, whitelist).await });

	let storage_mock = StorageMock::new();
	let celestia_mock = CelestiaMock::new();
	let _loop_jh = tokio::spawn(run(config, request_rx, storage_mock, celestia_mock));

	tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

	let tx = Transaction::test_only_new(b"test data".to_vec(), 1, 123);
	let txs = FullNodeTxs::new(vec![tx]);
	let batch_bytes = bcs::to_bytes(&txs).expect("Serialization failed");
	let signature = signing_key.sign(&batch_bytes);
	let signature = SigningSignature::try_from(&signature.to_bytes()[..]).unwrap();

	let serialized = serialize_full_node_batch(verifying_key, signature, batch_bytes);

	let connection_url = Url::parse(&format!("http://{}", grpc_address)).unwrap();
	let mut client = GrpcDaSequencerClient::try_connect(&connection_url)
		.await
		.expect("gRPC client connection failed");

	let request = BatchWriteRequest { data: serialized };
	let res = client.batch_write(request).await.expect("Failed to send the batch.");
	tracing::info!("{res:?}");
	assert!(!res.answer);

	tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
}

/// Submit a batch and sign it with a different key than the one declared in the whitelist.
/// The batch is rejected because the submitter public key is not part of the white list.
#[tokio::test]
async fn test_grpc_client_should_write_one_batch_with_a_wrong_verifying_key_in_whitelist() {
	let (request_tx, request_rx) = mpsc::channel(100);

	let config = DaSequencerConfig::default();
	let signing_key = generate_signing_key();
	let verifying_key = signing_key.verifying_key();

	// Create a white list with another signkey
	let other_signing_key = generate_signing_key();
	let other_verifying_key = other_signing_key.verifying_key();
	let whitelist = make_test_whitelist(vec![other_verifying_key]);

	// Start gprc server. Define a different address for each test.
	let grpc_address = "0.0.0.0:30705"
		.parse::<SocketAddr>()
		.expect("Bad da sequencer listener address.");
	let _grpc_jh =
		tokio::spawn(async move { run_server(grpc_address, request_tx, whitelist).await });

	let storage_mock = StorageMock::new();
	let celestia_mock = CelestiaMock::new();
	let _loop_jh = tokio::spawn(run(config, request_rx, storage_mock, celestia_mock));

	tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

	let tx = Transaction::test_only_new(b"test data".to_vec(), 1, 123);
	let txs = FullNodeTxs::new(vec![tx]);
	let batch_bytes = bcs::to_bytes(&txs).expect("Serialization failed");
	let signature = signing_key.sign(&batch_bytes);
	let signature = SigningSignature::try_from(&signature.to_bytes()[..]).unwrap();

	let serialized = serialize_full_node_batch(verifying_key, signature, batch_bytes);

	let connection_url = Url::parse(&format!("http://{}", grpc_address)).unwrap();
	let mut client = GrpcDaSequencerClient::try_connect(&connection_url)
		.await
		.expect("gRPC client connection failed");

	let request = BatchWriteRequest { data: serialized };
	let res = client.batch_write(request).await.expect("Failed to send the batch.");
	tracing::info!("{res:?}");
	assert!(!res.answer);

	tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
}

#[tokio::test]
async fn test_write_batch_grpc_main_loop_bad_signature() {
	let (request_tx, request_rx) = mpsc::channel(100);

	let config = DaSequencerConfig::default();
	let signing_key = generate_signing_key();
	let verifying_key = signing_key.verifying_key();
	let whitelist = make_test_whitelist(vec![verifying_key]);

	// Start gprc server. Define a different address for each test.
	let grpc_address = "0.0.0.0:30706"
		.parse::<SocketAddr>()
		.expect("Bad da sequencer listener address.");
	let _grpc_jh =
		tokio::spawn(async move { run_server(grpc_address, request_tx, whitelist).await });

	let storage_mock = StorageMock::new();
	let celestia_mock = CelestiaMock::new();
	let _loop_jh = tokio::spawn(run(config, request_rx, storage_mock, celestia_mock));

	tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

	let tx = Transaction::test_only_new(b"test data".to_vec(), 1, 123);
	let txs = FullNodeTxs::new(vec![tx]);
	let batch_bytes = bcs::to_bytes(&txs).expect("Serialization failed");

	// Create another key to sign
	let other_signing_key = generate_signing_key();
	let signature = other_signing_key.sign(&batch_bytes);
	let signature = SigningSignature::try_from(&signature.to_bytes()[..]).unwrap();

	let serialized = serialize_full_node_batch(verifying_key, signature, batch_bytes);

	let connection_url = Url::parse(&format!("http://{}", grpc_address)).unwrap();
	let mut client = GrpcDaSequencerClient::try_connect(&connection_url)
		.await
		.expect("gRPC client connection failed");

	let request = BatchWriteRequest { data: serialized };
	let res = client.batch_write(request).await.expect("Failed to send the batch.");
	tracing::info!("{res:?}");
	assert!(!res.answer);

	tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
}
