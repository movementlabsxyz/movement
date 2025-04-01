use crate::batch::{serialize_full_node_batch, FullNodeTxs};
use crate::run;
use crate::server::run_server;
use crate::tests::mock::{CelestiaMock, StorageMock};
use crate::whitelist::Whitelist;
use ed25519_dalek::Signature;
use movement_da_sequencer_client::{sign_batch, DaSequencerClient};
use movement_da_sequencer_config::DaSequencerConfig;
use movement_da_sequencer_proto::BatchWriteRequest;
use movement_types::transaction::Transaction;
use serial_test::serial;
use tokio::sync::mpsc;

pub mod mock;

#[serial]
#[tokio::test]
async fn test_write_batch_gprc_main_loop_happy_path() {
        // Create gRPC channel for test requests
        let (request_tx, request_rx) = mpsc::channel(100);

        // Create config and signer
        let config = DaSequencerConfig::default();
        let signing_key = config.signing_key.clone();
        let verifying_key = signing_key.verifying_key();

        // Add signer to whitelist before server starts
        {
                let mut whitelist = crate::whitelist::INSTANCE.lock().unwrap();
                whitelist.set_keys(vec![verifying_key]);
        }

        // Start gRPC server in background
        let grpc_address = config.movement_da_sequencer_listen_address;
        let _grpc_jh = tokio::spawn(async move {
                run_server(grpc_address, request_tx).await
        });

        // Start main loop (batch handler)
        let storage_mock = StorageMock::new();
        let celestia_mock = CelestiaMock::new();
        let _loop_jh = tokio::spawn(run(config.clone(), request_rx, storage_mock, celestia_mock));

        // Wait a moment to ensure server is up
        tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

        // Create and sign a test batch
        let tx = Transaction::test_only_new(b"test data".to_vec(), 1, 123);
        let txs = FullNodeTxs::new(vec![tx]);
        let batch_bytes = bcs::to_bytes(&txs).expect("Serialization failed");
        let signature = sign_batch(&batch_bytes, &signing_key);
        let serialized = serialize_full_node_batch(verifying_key, signature, batch_bytes);

        // Connect gRPC client and send the batch
        let connection_string = format!("http://127.0.0.1:{}", grpc_address.port());
        let mut client = DaSequencerClient::try_connect(&connection_string)
                .await
                .expect("gRPC client connection failed");

        let request = BatchWriteRequest { data: serialized };
        let res = client.batch_write(request).await.expect("Batch send failed");

        // Log response and assert success
        tracing::info!("{res:?}");
        assert!(res.answer);

        // Wait for block production (no verification yet)
        // TODO: verify the block produced contains the batch
        //       wait for stream/block implementation to land
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
}

#[tokio::test]
async fn test_write_batch_gprc_main_loop_unhappy_path() {
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
	// Wait the implementation of the stream of block.

	//wait at least one block production
	let _ = tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
}
