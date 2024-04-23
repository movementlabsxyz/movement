use sequencing_util::Sequencer;
use movement_types::{Block, Transaction, Id};
use mempool_util::{MempoolBlockOperations, MempoolTransactionOperations};
use std::{path::PathBuf, sync::Arc};
use tokio::sync::RwLock;
use move_rocks::RocksdbMempool;

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

impl Memseq<RocksdbMempool> {

    pub fn try_move_rocks(path : PathBuf) -> Result<Self, anyhow::Error> {
        let mempool = RocksdbMempool::try_new(path.to_str().ok_or(
            anyhow::anyhow!("PathBuf to str failed")
        )?)?;
        let mempool = Arc::new(RwLock::new(mempool));
        let parent_block = Arc::new(RwLock::new(Id::default()));
        Ok(Self::new(mempool, 10, parent_block, 1000))
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

        loop {

            let current_block_size = transactions.len() as u32;
            if current_block_size >= self.block_size {
                break;
            }

            for _ in 0..self.block_size - current_block_size {
                if let Some(transaction) = mempool.pop_transaction().await? {
                    transactions.push(transaction);
                } else {
                    break;
                }
            }

            // sleep to yield to other tasks and wait for more transactions
            tokio::time::sleep(std::time::Duration::from_millis(1)).await;

            now = std::time::Instant::now();
            if now > finish_by {
                break;
            }

        }
       
        if transactions.is_empty() {
            Ok(None)
        } else {
            Ok(Some(Block::new(
                Default::default(),
                self.parent_block.read().await.clone().to_vec(),
                transactions
            )))
        }

    }

}

#[cfg(test)]
pub mod test {

    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_memseq() -> Result<(), anyhow::Error>{

        let dir = tempdir()?;
        let path = dir.path().to_path_buf();
        let memseq = Memseq::try_move_rocks(path)?;

        let transaction = Transaction::new(vec![1, 2, 3]);
        memseq.publish(transaction.clone()).await?;
        
        Ok(())
    }

}