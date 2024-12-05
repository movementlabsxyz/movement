//! Task to process incoming transactions and write to DA

use maptos_dof_execution::SignedTransaction;
use movement_celestia_da_util::config::Config as LightNodeConfig;
use movement_da_light_node_client::MovementDaLightNodeClient;
use movement_da_light_node_proto::{BatchWriteRequest, BlobWrite};

use tokio::sync::mpsc;
use tracing::{info, warn};

use prost::Message;
use std::ops::ControlFlow;
use std::sync::atomic::AtomicU64;
use std::time::{Duration, Instant};

const LOGGING_UID: AtomicU64 = AtomicU64::new(0);

pub struct Task {
	transaction_receiver: mpsc::Receiver<(u64, SignedTransaction)>,
	da_light_node_client: MovementDaLightNodeClient,
	da_light_node_config: LightNodeConfig,
}

impl Task {
	pub(crate) fn new(
		transaction_receiver: mpsc::Receiver<(u64, SignedTransaction)>,
		da_light_node_client: MovementDaLightNodeClient,
		da_light_node_config: LightNodeConfig,
	) -> Self {
		Task { transaction_receiver, da_light_node_client, da_light_node_config }
	}

	pub async fn run(mut self) -> anyhow::Result<()> {
		while let ControlFlow::Continue(()) = self.spawn_write_next_transaction_batch().await? {}
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

		let batch_id = LOGGING_UID.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
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
					Some((application_priority, transaction)) => {
						info!(
							target : "movement_timing",
							batch_id = %batch_id,
							tx_hash = %transaction.committed_hash(),
							sender = %transaction.sender(),
							sequence_number = transaction.sequence_number(),
							"received transaction",
						);
						let serialized_aptos_transaction = bcs::to_bytes(&transaction)?;
						let movement_transaction = movement_types::transaction::Transaction::new(
							serialized_aptos_transaction,
							application_priority,
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
				target: "movement_timing",
				batch_id = %batch_id,
				transaction_count = transactions.len(),
				"built_batch_write"
			);
			let batch_write = BatchWriteRequest { blobs: transactions };
			let mut buf = Vec::new();
			batch_write.encode_raw(&mut buf);
			info!("batch_write size: {}", buf.len());
			// spawn the actual batch write request in the background
			let mut da_light_node_client = self.da_light_node_client.clone();
			tokio::spawn(async move {
				match da_light_node_client.batch_write(batch_write.clone()).await {
					Ok(_) => {
						info!(
							target: "movement_timing",
							batch_id = %batch_id,
							"batch_write_success"
						);
						return;
					}
					Err(e) => {
						warn!("failed to write batch to DA: {:?} {:?}", e, batch_id);
					}
				}
			});
		}

		Ok(Continue(()))
	}
}
