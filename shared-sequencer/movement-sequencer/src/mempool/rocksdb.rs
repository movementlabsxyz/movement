use super::{
    MempoolBlockOperations,
    MempoolTransactionOperations
};
use rocksdb::{DB, Options, ColumnFamilyDescriptor};
use std::sync::Arc;
use std::pin::Pin;
use serde_json;
use tokio::sync::RwLock;
use avalanche_types::ids::Id;
use futures::stream::Stream;
use crate::block::{Transaction, Block};

#[derive(Debug, Clone)]
pub struct RocksdbMempool {
    db: Arc<RwLock<DB>>
}

impl RocksdbMempool {
    pub fn new(path: &str) -> Self {
        let mut options = Options::default();
        options.create_if_missing(true);
        options.create_missing_column_families(true);

        let transaction_cf = ColumnFamilyDescriptor::new("transactions", Options::default());
        let block_cf = ColumnFamilyDescriptor::new("blocks", Options::default());

        let db = DB::open_cf_descriptors(&options, path, vec![transaction_cf, block_cf])
            .expect("Failed to open database with column families");

        RocksdbMempool {
            db: Arc::new(RwLock::new(db)),
        }
    }
}

#[tonic::async_trait]
impl MempoolTransactionOperations for RocksdbMempool {

    async fn has_transaction(&self, transaction_id: Id) -> Result<bool, anyhow::Error> {
        let db = self.db.read().await;
        let cf_handle = db.cf_handle("transactions").expect("CF handle not found");
        Ok(db.get_cf(&cf_handle, transaction_id.to_vec())?.is_some())
    }

    async fn add_transaction(&self, tx: Transaction) -> Result<(), anyhow::Error> {
        let serialized_tx = serde_json::to_vec(&tx)?;
        let db = self.db.write().await;
        let cf_handle = db.cf_handle("transactions").expect("CF handle not found");
        db.put_cf(&cf_handle, tx.id().to_vec(), &serialized_tx)?;
        Ok(())
    }

    async fn remove_transaction(&self, tx_id: Id) -> Result<(), anyhow::Error> {
        let db = self.db.write().await;
        let cf_handle = db.cf_handle("transactions").expect("CF handle not found");
        db.delete_cf(&cf_handle, tx_id.to_vec())?;
        Ok(())
    }

    async fn pop_transaction(&self, tx_id: Id) -> Result<Transaction, anyhow::Error> {
        let db = self.db.write().await;
        let cf_handle = db.cf_handle("transactions").expect("CF handle not found");
        let serialized_tx = db.get_cf(&cf_handle, tx_id.to_vec())?.expect("Transaction not found");
        let tx: Transaction = serde_json::from_slice(&serialized_tx)?;
        db.delete_cf(&cf_handle, tx_id.to_vec())?;
        Ok(tx)
    }

    async fn get_transaction(&self, tx_id: Id) -> Result<Transaction, anyhow::Error> {
        let db = self.db.read().await;
        let cf_handle = db.cf_handle("transactions").expect("CF handle not found");
        let serialized_tx = db.get_cf(&cf_handle, tx_id.to_vec())?.expect("Transaction not found");
        let tx: Transaction = serde_json::from_slice(&serialized_tx)?;
        Ok(tx)
    }

    async fn iter(&self) {
        // TODO Create tracking issue here.
        unimplemented!()
    }
    
}

#[tonic::async_trait]
impl MempoolBlockOperations for RocksdbMempool {
    async fn has_block(&self, block_id: Id) -> Result<bool, anyhow::Error> {
        let db = self.db.read().await;
        let cf_handle = db.cf_handle("blocks").expect("CF handle not found");
        Ok(db.get_cf(&cf_handle, block_id.to_vec())?.is_some())
    }

    async fn add_block(&self, block: Block) -> Result<(), anyhow::Error> {
        let serialized_block = serde_json::to_vec(&block)?;
        let db = self.db.write().await;
        let cf_handle = db.cf_handle("blocks").expect("CF handle not found");
        db.put_cf(&cf_handle, block.id().to_vec(), &serialized_block)?;
        Ok(())
    }

    async fn remove_block(&self, block_id: Id) -> Result<(), anyhow::Error> {
        let db = self.db.write().await;
        let cf_handle = db.cf_handle("blocks").expect("CF handle not found");
        db.delete_cf(&cf_handle, block_id.to_vec())?;
        Ok(())
    }

    async fn pop_block(&self, block_id: Id) -> Result<Block, anyhow::Error> {
        let db = self.db.write().await;
        let cf_handle = db.cf_handle("blocks").expect("CF handle not found");
        let serialized_block = db.get_cf(&cf_handle, block_id.to_vec())?.expect("Block not found");
        let block: Block = serde_json::from_slice(&serialized_block)?;
        db.delete_cf(&cf_handle, block_id.to_vec())?;
        Ok(block)
    }

    async fn get_block(&self, block_id: Id) -> Result<Block, anyhow::Error> {
        let db = self.db.read().await;
        let cf_handle = db.cf_handle("blocks").expect("CF handle not found");
        let serialized_block = db.get_cf(&cf_handle, block_id.to_vec())?.expect("Block not found");
        let block: Block = serde_json::from_slice(&serialized_block)?;
        Ok(block)
    }
   
}