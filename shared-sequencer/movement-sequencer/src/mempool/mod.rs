use crate::block::{
    Transaction,
    Block
};
pub mod rocksdb;

#[tonic::async_trait]
pub trait MempoolTransactionOperations {
    
    /// Checks whether a transaction exists in the mempool.
    async fn has_transaction(&self, transaction_id : avalanche_types::ids::Id) -> Result<bool, anyhow::Error>;

    /// Adds a transaction to the mempool.
    async fn add_transaction(&self, tx: Transaction) -> Result<(), anyhow::Error>;

    /// Removes a transaction from the mempool.
    async fn remove_transaction(&self, transaction_id: avalanche_types::ids::Id) -> Result<(), anyhow::Error>;

    /// Pops transaction from the mempool.
    async fn pop_transaction(&self, transaction_id: avalanche_types::ids::Id) -> Result<Transaction, anyhow::Error>;
 
    /// Gets a transaction from the mempool.
    async fn get_transaction(&self, transaction_id: avalanche_types::ids::Id) -> Result<Transaction, anyhow::Error>;

    /// Provides well-ordered transaction iterable
    async fn iter(&self) -> Result<impl Iterator<Item = Transaction>, anyhow::Error>;

}

#[tonic::async_trait]
pub trait MempoolBlockOperations {
    
    /// Checks whether a block exists in the mempool.
    async fn has_block(&self, block_id : avalanche_types::ids::Id) -> Result<bool, anyhow::Error>;

    /// Adds a block to the mempool.
    async fn add_block(&self, block: Block) -> Result<(), anyhow::Error>;

    /// Removes a block from the mempool.
    async fn remove_block(&self, block_id: avalanche_types::ids::Id) -> Result<(), anyhow::Error>;

    /// Pops block from the mempool.
    async fn pop_block(&self, block_id: avalanche_types::ids::Id) -> Result<Block, anyhow::Error>;
 
    /// Gets a block from the mempool.
    async fn get_block(&self, block_id: avalanche_types::ids::Id) -> Result<Block, anyhow::Error>;

}

#[derive(Debug, Clone)]
pub struct Mempool {
 

}