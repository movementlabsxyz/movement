use avalanche_types::{
    subnet,
    choices,
    ids::{self, Id},
    subnet::rpc::consensus::snowman::{
        Decidable,
        Block as Blockable
    }
};
use movement_sdk::DataAvailabilityLayer;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::io;
use std::collections::HashMap;
use serde::{de::DeserializeOwned, Serialize};
use serde_json;

pub struct AvalancheDataAvailabilityLayer<Block> {
    pub db: Arc<RwLock<Box<dyn subnet::rpc::database::Database + Send + Sync>>>,
    pub verified_blocks: Arc<RwLock<HashMap<ids::Id, Block>>>,
}

impl <Block : Serialize + DeserializeOwned> AvalancheDataAvailabilityLayer<Block> {

    pub fn get_block_with_status_key(
        block : &Block
    ) -> Result<ids::Id, anyhow::Error> {
        let bytes = serde_json::to_vec(&block)?;
        let id = ids::Id::from_slice(&bytes.as_slice());
        Ok(id)
    }

    pub async fn write_block(
        &self,
        block: &Block
    ) -> Result<(), anyhow::Error> {
        let mut db = self.db.write().await;
        let key = Self::get_block_with_status_key(block)?.to_vec().as_slice();
        let value = serde_json::to_vec(block)?.as_slice();
        db.put(key, value).await?;
        Ok(())
    }

}

impl <Block> AvalancheDataAvailabilityLayer<Block> {

    pub fn new(
        db: Arc<RwLock<Box<dyn subnet::rpc::database::Database + Send + Sync>>>,
        verified_blocks: Arc<RwLock<HashMap<ids::Id, Block>>>,
    ) -> Self {
        Self {
            db,
            verified_blocks,
        }
    }

}

#[async_trait::async_trait]
impl <Block, BlockId> DataAvailabilityLayer for AvalancheDataAvailabilityLayer<Block> {

    type Block = Block;
    type BlockId = BlockId;

    /// Gets the next block from the previous layer.
    async fn get_next_block(
        &self
    ) -> Result<Option<Self::Block>, anyhow::Error> {
        todo!()
    }

    /// Accepts a block, effectively sending it to the next layer or place retrievable from the next layer, i.e., the execution layer.
    async fn accept_block(
        &self,
        block: Self::Block
    ) -> Result<(), anyhow::Error> {
        todo!()
    }

    /// Rejects a block (sometimes this won't be used).
    async fn reject_block(
        &self,
        block: Self::Block
    ) -> Result<(), anyhow::Error> {
        todo!()
    }

    /// Gets a block that was either accepted or rejected by the data availability layer.
    async fn get_block(
        &self,
        block_id: Self::BlockId
    ) -> Result<Option<Self::Block>, anyhow::Error> {
        todo!()
    }

}