//! Task to process incoming transactions and write to DA

use maptos_dof_execution::SignedTransaction;
use maptos_execution_util::config::Config as MaptosConfig;
use movement_da_light_node_client::MovementDaLightNodeClient;
use movement_da_light_node_proto::{BatchWriteRequest, BlobWrite};

use tokio::sync::mpsc;
use tracing::{debug, info, warn};

use prost::Message;
use std::ops::ControlFlow;
use std::sync::atomic::AtomicU64;

const LOGGING_UID: AtomicU64 = AtomicU64::new(0);

pub struct Task {
	transaction_receiver: mpsc::Receiver<Vec<(u64, SignedTransaction)>>,
	da_light_node_client: MovementDaLightNodeClient,
	#[allow(dead_code)]
	maptos_config: MaptosConfig,
}

impl Task {
	pub(crate) fn new(
		transaction_receiver: mpsc::Receiver<Vec<(u64, SignedTransaction)>>,
		da_light_node_client: MovementDaLightNodeClient,
		maptos_config: MaptosConfig,
	) -> Self {
		Task { transaction_receiver, da_light_node_client, maptos_config }
	}

	pub async fn run(mut self) -> anyhow::Result<()> {
		while let ControlFlow::Continue(()) = self.spawn_write_next_transaction_batch().await? {}
		Ok(())
	}

	/// Receives the next pre-built transaction batch from the channel and submits it to the DA node.
	async fn spawn_write_next_transaction_batch(
		&mut self,
	) -> Result<ControlFlow<(), ()>, anyhow::Error> {
		use ControlFlow::{Break, Continue};
	
		let Some(batch) = self.transaction_receiver.recv().await else {
			return Ok(Break(())); // channel closed
		};
	
		let batch_id = LOGGING_UID.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
	
		let transactions = batch
			.into_iter()
			.map(|(priority, transaction)| {
				debug!(
					target: "movement_timing",
					batch_id = %batch_id,
					tx_hash = %transaction.committed_hash(),
					sender = %transaction.sender(),
					sequence_number = transaction.sequence_number(),
					"Tx ingress received transaction",
				);
				let serialized = bcs::to_bytes(&transaction)?;
				let movement_transaction = movement_types::transaction::Transaction::new(
					serialized,
					priority,
					transaction.sequence_number(),
				);
				let encoded = serde_json::to_vec(&movement_transaction)?;
				Ok(BlobWrite { data: encoded })
			})
			.collect::<Result<Vec<_>, anyhow::Error>>()?;
	
		let batch_write = BatchWriteRequest { blobs: transactions };
		let mut buf = Vec::new();
		batch_write.encode_raw(&mut buf);
		info!("built_batch_write batch_id={} size={}", batch_id, buf.len());
	
		let mut da_client = self.da_light_node_client.clone();
		tokio::spawn(async move {
			if let Err(e) = da_client.batch_write(batch_write).await {
				warn!("batch_write failed batch_id={} error={:?}", batch_id, e);
				// This is where retry logic or panic can be hooked in later
			} else {
				info!(target: "movement_timing", batch_id = %batch_id, "batch_write_success");
			}
		});
	
		Ok(Continue(()))
	}
}
