use crate::batch::serialize_full_node_batch;
use crate::batch::{FullNodeTxs, serialize_full_node_batch};
use crate::run;
use crate::server::run_server;
use crate::tests::mock::CelestiaMock;
use crate::tests::mock::{StorageMock, CelestiaMock};
use movement_da_sequencer_client::{sign_batch, DaSequencerClient};
use movement_da_sequencer_client::DaSequencerClient;
use movement_da_sequencer_config::DaSequencerConfig;
use movement_da_sequencer_proto::BatchWriteRequest;
use movement_types::transaction::Transaction;
use tokio::sync::mpsc;

mod mock;

#[tokio::test]
async fn test_write_batch_gprc_main_loop() {
	let (request_tx, request_rx) = mpsc::channel(100);

	let config = DaSequencerConfig::default();
	let signing_key = config.signing_key.clone();
	let verifying_key = signing_key.verifying_key();

	// Start gprc server
	let grpc_address = config.movement_da_sequencer_listen_address;
	let grpc_jh = tokio::spawn(async move { run_server(grpc_address, request_tx).await });

	//start main loop
	let storage_mock = StorageMock::new();
	let celestia_mock = CelestiaMock::new();
	let loop_jh = tokio::spawn(run(config, request_rx, storage_mock, celestia_mock));

	//need to wait the server is started before connecting
	let _ = tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

	let tx = Transaction::test_only_new(b"test data".to_vec(), 1, 123);
	let tx_id = tx.id();

	let txs = FullNodeTxs::new(vec![tx]);

	//sign batch
	let batch_bytes = bcs::to_bytes(&txs).expect("Serialization failed");
	let signature = sign_batch(&batch_bytes, &signing_key);

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
	assert!(res.answer);

	//TODO verify the block produced contains the batch.
	// Wait the implementation of the stream of block.

	//wait at least one block production
	let _ = tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
}
