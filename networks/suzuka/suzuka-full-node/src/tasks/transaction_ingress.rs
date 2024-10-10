//! Task to process incoming transactions and write to DA

use m1_da_light_node_client::{BatchWriteRequest, BlobWrite, LightNodeServiceClient};
use m1_da_light_node_util::config::Config as LightNodeConfig;
use maptos_dof_execution::SignedTransaction;

use tokio::sync::mpsc;
use tracing::{info, info_span, warn, Instrument};

use std::ops::ControlFlow;
use std::sync::atomic::{self, AtomicU64};
use std::time::{Duration, Instant};

const LOGGING_UID: AtomicU64 = AtomicU64::new(0);

pub struct Task {
	transaction_receiver: mpsc::Receiver<SignedTransaction>,
	da_light_node_client: LightNodeServiceClient<tonic::transport::Channel>,
	da_light_node_config: LightNodeConfig,
}

impl Task {
	pub(crate) fn new(
		transaction_receiver: mpsc::Receiver<SignedTransaction>,
		da_light_node_client: LightNodeServiceClient<tonic::transport::Channel>,
		da_light_node_config: LightNodeConfig,
	) -> Self {
		Task { transaction_receiver, da_light_node_client, da_light_node_config }
	}

	pub async fn run(mut self) -> anyhow::Result<()> {
		loop {
			let batch_id = LOGGING_UID.fetch_add(1, atomic::Ordering::Relaxed);
			let span =
				info_span!(target: "movement_telemetry", "write_batch", batch_id = %batch_id);
			if let ControlFlow::Break(()) =
				self.spawn_write_next_transaction_batch().instrument(span).await?
			{
				break;
			}
		}
		Ok(())
	}

	/// Constructs a batch of transactions then spawns the write request to the DA in the background.
	async fn spawn_write_next_transaction_batch(
		&mut self,
	) -> Result<ControlFlow<(), ()>, anyhow::Error> {
		use ControlFlow::{Break, Continue};

		// limit the total time batching transactions
		let start = Instant::now();
		let (_, half_building_time) = self.da_light_node_config.try_block_building_parameters()?;

		let mut transactions = Vec::new();

		loop {
			let remaining = match half_building_time.checked_sub(start.elapsed().as_millis() as u64)
			{
				Some(remaining) => remaining,
				None => {
					// we have exceeded the half building time
					break;
				}
			};

			match tokio::time::timeout(
				Duration::from_millis(remaining),
				self.transaction_receiver.recv(),
			)
			.await
			{
				Ok(transaction) => match transaction {
					Some(transaction) => {
						info!(
							target: "movement_telemetry",
							tx_hash = %transaction.committed_hash(),
							sender = %transaction.sender(),
							sequence_number = transaction.sequence_number(),
							"received_transaction",
						);
						let serialized_aptos_transaction = serde_json::to_vec(&transaction)?;
						let movement_transaction = movement_types::transaction::Transaction::new(
							serialized_aptos_transaction,
							transaction.sequence_number(),
						);
						let serialized_transaction = serde_json::to_vec(&movement_transaction)?;
						transactions.push(BlobWrite { data: serialized_transaction });
					}
					None => {
						// The transaction stream is closed, terminate the task.
						return Ok(Break(()));
					}
				},
				Err(_) => {
					break;
				}
			}
		}

		if transactions.len() > 0 {
			info!(
				target: "movement_telemetry",
				transaction_count = transactions.len(),
				"built_batch_write"
			);
			let batch_write = BatchWriteRequest { blobs: transactions };
			// spawn the actual batch write request in the background
			let mut da_light_node_client = self.da_light_node_client.clone();
			tokio::spawn(async move {
				if let Err(e) = da_light_node_client.batch_write(batch_write).await {
					warn!("failed to write batch to DA: {:?}", e);
				}
			});
		}

		Ok(Continue(()))
	}
}
