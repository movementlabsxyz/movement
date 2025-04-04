use crate::{
	batch::*,
	run,
	server::{run_server, GrpcRequests},
	tests::{
		mock::{CelestiaMock, StorageMock},
		whitelist::make_test_whitelist,
	},
	whitelist::Whitelist,
};
use ed25519_dalek::Signature;
use movement_da_sequencer_client::{sign_batch, DaSequencerClient};
use movement_da_sequencer_config::DaSequencerConfig;
use movement_da_sequencer_proto::BatchWriteRequest;
use movement_types::transaction::Transaction;
use serial_test::serial;
use std::{net::TcpListener, sync::Arc};
use tokio::{
	sync::{mpsc, RwLock},
	time::Duration,
};
use tracing_subscriber;

pub mod mock;
pub mod whitelist;

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
