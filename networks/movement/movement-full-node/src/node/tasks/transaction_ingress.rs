//! Task to process a single batch of transactions and write to DA

use maptos_dof_execution::SignedTransaction;
use maptos_execution_util::config::Config as MaptosConfig;
use movement_da_light_node_client::MovementDaLightNodeClient;
use movement_da_light_node_proto::{BatchWriteRequest, BlobWrite};

use tokio::sync::mpsc;
use tracing::{debug, info};

use prost::Message;
use std::sync::atomic::{AtomicU64, Ordering};

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
                Task {
                        transaction_receiver,
                        da_light_node_client,
                        maptos_config,
                }
        }

        pub async fn run(mut self) -> anyhow::Result<()> {
                if let Some(batch) = self.transaction_receiver.recv().await {
                        self.process_batch(batch).await?;
                }
                Ok(())
        }

        async fn process_batch(
                &mut self,
                batch: Vec<(u64, SignedTransaction)>,
        ) -> anyhow::Result<()> {
                let batch_id = LOGGING_UID.fetch_add(1, Ordering::SeqCst);

                let blobs = batch
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

                let batch_write = BatchWriteRequest { blobs };
                let mut buf = Vec::new();
                batch_write.encode_raw(&mut buf);
                info!(
                        "built_batch_write batch_id={} size={}",
                        batch_id,
                        buf.len()
                );

                let batch_write_clone = batch_write.clone();
                let mut da_client = self.da_light_node_client.clone();

                Ok(())
        }
}
