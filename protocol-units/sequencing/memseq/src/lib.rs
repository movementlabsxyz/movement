use sequencing_util::Sequencer;
use movement_types::{Block, Transaction, Id};
use mempool_util::{MempoolBlockOperations, MempoolTransactionOperations};
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct Memseq<T : MempoolBlockOperations + MempoolTransactionOperations> {
    pub mempool : Arc<RwLock<T>>,
    pub block_size : u32,
    pub parent_block : Arc<RwLock<Id>>,
    pub building_time_ms : u64,
}

impl <T : MempoolBlockOperations + MempoolTransactionOperations> Memseq<T> {
    pub fn new(mempool : Arc<RwLock<T>>, block_size : u32, parent_block : Arc<RwLock<Id>>, building_time_ms : u64) -> Self {
        Self {
            mempool,
            block_size,
            parent_block,
            building_time_ms
        }
    }
}

impl <T : MempoolBlockOperations + MempoolTransactionOperations> Sequencer for Memseq<T> {

    async fn publish(&self, transaction: Transaction) -> Result<(), anyhow::Error> {
        let mempool = self.mempool.read().await;
        mempool.add_transaction(transaction).await?;
        Ok(())
    }

    async fn wait_for_next_block(&self) -> Result<Option<Block>, anyhow::Error> {
        
        let mempool = self.mempool.read().await;
        let mut transactions = Vec::new();

        let mut now = std::time::Instant::now();
        let finish_by = now + std::time::Duration::from_millis(self.building_time_ms);
        for i in 0..self.block_size {

            if let Some(transaction) = mempool.pop_transaction().await? {
                transactions.push(transaction);
            } else {
                if i == 0 {
                    return Ok(None);
                }
                break;
            }

            now = std::time::Instant::now();
            if now > finish_by {
                break;
            }

        }

        Ok(Some(Block::new(
            Default::default(),
            self.parent_block.read().await.clone().to_vec(),
            transactions
        )))

    }

}