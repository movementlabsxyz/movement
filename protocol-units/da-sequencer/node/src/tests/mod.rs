use crate::batch::*;
use crate::run;
use crate::server::{run_server, GrpcRequests};
use crate::tests::mock::{CelestiaMock, StorageMock};
use crate::whitelist::Whitelist;
use ed25519_dalek::Signature;
use movement_da_sequencer_client::{sign_batch, DaSequencerClient};
use movement_da_sequencer_config::DaSequencerConfig;
use movement_da_sequencer_proto::BatchWriteRequest;
use movement_types::transaction::Transaction;
use serial_test::serial;
use std::net::TcpListener;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::sync::RwLock;
use tokio::time::Duration;
use tracing_subscriber;

pub mod mock;

fn make_test_whitelist(keys: Vec<ed25519_dalek::VerifyingKey>) -> Whitelist {
	Whitelist::from_keys(keys)
}

#[serial]
#[tokio::test]
async fn test_sign_and_validate_batch_passes_with_whitelisted_signer() {
	let _ = tracing_subscriber::fmt()
		.with_max_level(tracing::Level::INFO)
		.with_test_writer()
		.try_init();

	let config = DaSequencerConfig::default();
	let signing_key = config.signing_key;
	let verifying_key = signing_key.verifying_key();

	let whitelist = Whitelist::from_keys(vec![verifying_key]);

	let txs = FullNodeTxs(vec![
		Transaction::new(b"hello".to_vec(), 0, 1),
		Transaction::new(b"world".to_vec(), 0, 2),
	]);

	let batch_bytes = bcs::to_bytes(&txs).expect("Serialization failed");
	let signature = sign_batch(&batch_bytes, &signing_key);
	let serialized =
		serialize_full_node_batch(verifying_key, signature.clone(), batch_bytes.clone());

	let (deserialized_key, deserialized_sig, deserialized_data) =
		deserialize_full_node_batch(serialized).expect("Deserialization failed");

	let raw_batch = DaBatch {
		data: RawData { data: deserialized_data },
		signature: deserialized_sig,
		signer: deserialized_key,
		timestamp: chrono::Utc::now().timestamp_micros() as u64,
	};

	let validated = validate_batch(raw_batch, &whitelist).expect("Batch should validate");
	assert_eq!(validated.data.0, txs.0);
}

#[serial]
#[tokio::test]
async fn test_sign_and_validate_batch_fails_with_non_whitelisted_signer() {
	let _ = tracing_subscriber::fmt()
		.with_max_level(tracing::Level::INFO)
		.with_test_writer()
		.try_init();

	let whitelist = Whitelist::from_keys(vec![]); // empty whitelist

	let config = DaSequencerConfig::default();
	let signing_key = config.signing_key;
	let verifying_key = signing_key.verifying_key();

	let txs = FullNodeTxs(vec![
		Transaction::new(b"hello".to_vec(), 0, 1),
		Transaction::new(b"world".to_vec(), 0, 2),
	]);

	let batch_bytes = bcs::to_bytes(&txs).expect("Serialization failed");
	let signature = sign_batch(&batch_bytes, &signing_key);
	let serialized =
		serialize_full_node_batch(verifying_key, signature.clone(), batch_bytes.clone());

	let (deserialized_key, deserialized_sig, deserialized_data) =
		deserialize_full_node_batch(serialized).expect("Deserialization failed");

	let raw_batch = DaBatch {
		data: RawData { data: deserialized_data },
		signature: deserialized_sig,
		signer: deserialized_key,
		timestamp: chrono::Utc::now().timestamp_micros() as u64,
	};

	let result = validate_batch(raw_batch, &whitelist);
	assert!(matches!(result, Err(crate::error::DaSequencerError::InvalidSigner)));
}

#[tokio::test]
#[serial]
async fn test_write_batch_grpc_main_loop_happy_path() {
	let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
	let grpc_address = listener.local_addr().unwrap();
	drop(listener);

	let config =
		DaSequencerConfig { grpc_listen_address: grpc_address, ..DaSequencerConfig::default() };

	let signing_key = config.signing_key.clone();
	let verifying_key = signing_key.verifying_key();
	let whitelist = make_test_whitelist(vec![verifying_key.clone()]);
	let (request_tx, request_rx) = mpsc::channel(100);

	let grpc_task = tokio::spawn(run_server(grpc_address, request_tx, whitelist.clone()));
	let main_loop = tokio::spawn(async move {
		let storage = StorageMock::new();
		let da = CelestiaMock::new();
		run(config, request_rx, storage, da).await.unwrap();
	});

	tokio::time::sleep(Duration::from_millis(300)).await;

	let addr = format!("http://{}", grpc_address);
	let mut client = DaSequencerClient::try_connect(&addr).await.expect("Failed to connect");

	let tx = Transaction::test_only_new(b"abc".to_vec(), 1, 123);
	let txs = FullNodeTxs::new(vec![tx]);
	let batch_bytes = bcs::to_bytes(&txs).unwrap();
	let sig = sign_batch(&batch_bytes, &signing_key);
	let data = serialize_full_node_batch(verifying_key, sig, batch_bytes);

	let res = client.batch_write(BatchWriteRequest { data }).await;
	let response = res.expect("batch_write failed");
	assert!(response.answer);

	grpc_task.abort();
	let _ = grpc_task.await;

	main_loop.abort();
	let _ = main_loop.await;
}

#[tokio::test]
async fn test_write_batch_grpc_main_loop_unhappy_path() {
	let (request_tx, request_rx) = mpsc::channel(100);

	let config = DaSequencerConfig::default();
	let signing_key = config.signing_key.clone();
	let verifying_key = signing_key.verifying_key();

	let whitelist = make_test_whitelist(vec![]);
	let whitelist_clone = whitelist.clone();

	let grpc_address = config.grpc_listen_address;
	let _grpc_jh =
		tokio::spawn(async move { run_server(grpc_address, request_tx, whitelist_clone).await });

	let storage_mock = StorageMock::new();
	let celestia_mock = CelestiaMock::new();
	let _loop_jh = tokio::spawn(run(config, request_rx, storage_mock, celestia_mock));

	tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

	let tx = Transaction::test_only_new(b"test data".to_vec(), 1, 123);
	let txs = FullNodeTxs::new(vec![tx]);
	let batch_bytes = bcs::to_bytes(&txs).expect("Serialization failed");
	let signature = Signature::from_bytes(&[0; 64]);
	let serialized = serialize_full_node_batch(verifying_key, signature, batch_bytes);

	let connection_string = format!("http://127.0.0.1:{}", grpc_address.port());
	let mut client = DaSequencerClient::try_connect(&connection_string)
		.await
		.expect("gRPC client connection failed");

	let request = BatchWriteRequest { data: serialized };
	let res = client.batch_write(request).await.expect("Failed to send the batch.");
	tracing::info!("{res:?}");
	assert!(!res.answer);

	tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
}
