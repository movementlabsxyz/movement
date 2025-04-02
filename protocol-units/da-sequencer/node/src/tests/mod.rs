use crate::batch::{serialize_full_node_batch, FullNodeTxs};
use crate::run;
use crate::server::run_server;
use crate::tests::mock::mock_wait_and_get_next_block;
use crate::tests::mock::mock_write_new_batch;
use crate::tests::mock::{CelestiaMock, StorageMock};
use ed25519_dalek::Signature;
use futures::StreamExt;
use movement_da_sequencer_client::DaSequencerClient;
use movement_da_sequencer_config::DaSequencerConfig;
use movement_da_sequencer_proto::blob_response::BlobType;
use movement_da_sequencer_proto::BatchWriteRequest;
use movement_da_sequencer_proto::StreamReadFromHeightRequest;
use movement_types::transaction::Transaction;
use std::net::SocketAddr;
use tokio::sync::mpsc;

pub mod mock;

#[tokio::test]
async fn test_write_batch_gprc_main_loop_happy_path() {
	let (request_tx, request_rx) = mpsc::channel(100);

	let config = DaSequencerConfig::default();
	let signing_key = config.signing_key.clone();
	let verifying_key = signing_key.verifying_key();

	// Start gprc server
	// Start gprc server. Define a different address for each test.
	let grpc_address = "0.0.0.0:30700"
		.parse::<SocketAddr>()
		.expect("Bad da sequencer listener address.");
	let grpc_jh = tokio::spawn(async move { run_server(grpc_address, request_tx).await });

	//start main loop
	let storage_mock = StorageMock::new();
	let celestia_mock = CelestiaMock::new();
	let loop_jh = tokio::spawn(run(config, request_rx, storage_mock, celestia_mock));

	//need to wait the server is started before connecting
	let _ = tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

	let connection_string = format!("http://127.0.0.1:{}", grpc_address.port());
	let mut client = DaSequencerClient::try_connect(&connection_string)
		.await
		.expect("gRPC client connection failed.");

	mock_write_new_batch(&mut client, &signing_key, verifying_key).await;

	//verify the block produced contains the batch.
	//Register to block stream
	let connection_string = format!("http://127.0.0.1:{}", grpc_address.port());
	let mut client = DaSequencerClient::try_connect(&connection_string)
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
async fn test_write_batch_gprc_main_loop_happy_path_unhappy_path() {
	let (request_tx, request_rx) = mpsc::channel(100);

	let config = DaSequencerConfig::default();
	let signing_key = config.signing_key.clone();
	let verifying_key = signing_key.verifying_key();

	// Start gprc server. Define a different address for each test.
	let grpc_address = "0.0.0.0:30701"
		.parse::<SocketAddr>()
		.expect("Bad da sequencer listener address.");

	let grpc_jh = tokio::spawn(async move { run_server(grpc_address, request_tx).await });

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

	// Serialize full node batch into raw bytes
	let serialized =
		serialize_full_node_batch(verifying_key, signature.clone(), batch_bytes.clone());

	//send the bacth using the grpc client
	let connection_string = format!("http://127.0.0.1:{}", grpc_address.port());
	let mut client = DaSequencerClient::try_connect(&connection_string)
		.await
		.expect("gRPC client connection failed.");

	let request = BatchWriteRequest { data: serialized };
	let res = client.batch_write(request).await.expect("Failed to send the batch.");
	tracing::info!("{res:?}",);
	//return false because of the signature.
	assert!(!res.answer);

	//TODO verify no block has been produced.
	//Register to block stream
	let connection_string = format!("http://127.0.0.1:{}", grpc_address.port());
	let mut client = DaSequencerClient::try_connect(&connection_string)
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
async fn test_produc_block_and_stream() {
	// let _ = tracing_subscriber::fmt()
	// 	.with_max_level(tracing::Level::INFO)
	// 	.with_test_writer()
	// 	.try_init();

	let (request_tx, request_rx) = mpsc::channel(100);

	let mut config = DaSequencerConfig::default();
	//update config to generate faster heartbeat.
	config.movement_da_sequencer_stream_heartbeat_interval_sec = 1;
	let signing_key = config.signing_key.clone();
	let verifying_key = signing_key.verifying_key();

	// Start gprc server
	// Start gprc server. Define a different address for each test.
	let grpc_address = "0.0.0.0:30702"
		.parse::<SocketAddr>()
		.expect("Bad da sequencer listener address.");
	let grpc_jh = tokio::spawn(async move { run_server(grpc_address, request_tx).await });

	//start main loop
	let storage_mock = StorageMock::new();
	let celestia_mock = CelestiaMock::new();
	let loop_jh = tokio::spawn(run(config, request_rx, storage_mock, celestia_mock));

	//need to wait the server is started before connecting
	let _ = tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

	//Register to block stream
	let connection_string = format!("http://127.0.0.1:{}", grpc_address.port());
	let mut client = DaSequencerClient::try_connect(&connection_string)
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
		match block.response.unwrap().blob_type {
			Some(BlobType::Heartbeat(_)) => (),
			_ => panic!("Error get a genesis block at height 0 "),
		}
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
	let mut client2 = DaSequencerClient::try_connect(&connection_string)
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

	//detect the heartbeat
	match tokio::time::timeout(std::time::Duration::from_secs(1), block_stream.next()).await {
		Ok(Some(Ok(block))) => match block.response.unwrap().blob_type {
			Some(BlobType::Heartbeat(_)) => (),
			_ => panic!("Not a heartbeat."),
		},
		_ => panic!("No hearbeat produced"),
	};

	grpc_jh.abort();
	loop_jh.abort();
}
