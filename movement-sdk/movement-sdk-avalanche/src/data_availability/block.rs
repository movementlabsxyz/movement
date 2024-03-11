use avalanche_types::{
    subnet,
    choices,
    ids::{self, Id},
    subnet::rpc::consensus::snowman::{
        Decidable,
        Block as Blockable
    }
};
use movement_sdk::{Layer, DataAvailabilityLayer};
use std::sync::Arc;
use tokio::sync::RwLock;
use std::io;

#[derive(Debug, Clone)]
pub struct AvalancheBlock<
    Block : Send + Sync, 
    BlockId : Send + Sync,
    DA : DataAvailabilityLayer<Block = Block, BlockId = BlockId>
>{
    pub inner_block: Arc<Block>,
    pub data_availability_layer: Arc<RwLock<DA>>,
}

impl <
    Block : Send + Sync, 
    BlockId : Send + Sync,
    DA : DataAvailabilityLayer<Block = Block, BlockId = BlockId>
> AvalancheBlock<Block, BlockId, DA> {

    pub fn new(
        inner_block: Block,
        data_availability_layer: Arc<RwLock<DA>>,
    ) -> Self {
        Self {
            inner_block : Arc::new(RwLock::new(inner_block)),
            data_availability_layer,
        }
    }

}

impl <
    Block : Send + Sync, 
    BlockId : Send + Sync,
    DA : DataAvailabilityLayer<Block = Block, BlockId = BlockId>
> Into<AvalancheBlock<Block, BlockId, DA>> for (Block, Arc<RwLock<DA>>) {
    fn into(self) -> AvalancheBlock<Block, BlockId, DA> {
        AvalancheBlock::new(self.0, self.1)
    }
}

#[async_trait::async_trait]
impl <
    Block : Decidable + Clone + Send + Sync, 
    BlockId : Send + Sync,
    DA : DataAvailabilityLayer<Block = Block, BlockId = BlockId> + Sync + Send
> Decidable for AvalancheBlock<Block, BlockId, DA> {
    
    async fn id(&self) -> ids::Id {
        self.inner_block.id().await
    }

    async fn status(&self) -> choices::status::Status {
        self.inner_block.status().await
    }

    async fn accept(&mut self) -> io::Result<()> {
        let data_availability_layer = self.data_availability_layer.read().await;
        let block_copy = self.inner_block.read().await.clone();
        let result = data_availability_layer.accept_block(block_copy).await;
        result.map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        Ok(())
    }
    
    async fn reject(&mut self) -> io::Result<()> {
        let data_availability_layer = self.data_availability_layer.read().await;
        let block_copy = self.inner_block.read().await.clone();
        let result  = data_availability_layer.reject_block(block_copy).await;
        result.map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        Ok(())
    }

}

#[cfg(test)]
mod test {

    use super::*;

    #[derive(Debug, Clone)]
    pub struct MyBlock(String);

    #[async_trait::async_trait]
    impl Decidable for MyBlock {
        async fn id(&self) -> ids::Id {
            ids::Id::sha256(self.0.as_bytes())
        }

        async fn status(&self) -> choices::status::Status {
            choices::status::Status::Unknown
        }

        async fn accept(&mut self) -> io::Result<()> {
            Ok(())
        }
        
        async fn reject(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    #[derive(Debug, Clone)]
    pub struct MyDataAvailabilityLayer;

    impl Layer for MyDataAvailabilityLayer {}

    #[async_trait::async_trait]
    impl DataAvailabilityLayer for MyDataAvailabilityLayer {
        type Block = MyBlock;
        type BlockId = ids::Id;

        async fn get_next_block(
            &self
        ) -> Result<Option<Self::Block>, anyhow::Error> {
            Ok(None)
        }

        async fn accept_block(
            &self,
            block: Self::Block
        ) -> Result<(), anyhow::Error> {
            Ok(())
        }

        async fn reject_block(
            &self,
            block: Self::Block
        ) -> Result<(), anyhow::Error> {
            Ok(())
        }

        async fn get_block(
            &self,
            block_id: Self::BlockId
        ) -> Result<Option<Self::Block>, anyhow::Error> {
            Ok(None)
        }
    }

    #[tokio::test]
    async fn test_avalanche_block() {
        let data_availability_layer = Arc::new(RwLock::new(MyDataAvailabilityLayer));
        let block = MyBlock("hello".to_string());
        let avalanche_block = AvalancheBlock::new(block, data_availability_layer);
        let id = avalanche_block.id().await;
        assert_eq!(id, ids::Id::sha256("hello".as_bytes()));
        let status = avalanche_block.status().await;
        assert_eq!(status, choices::status::Status::Unknown);
        let result = avalanche_block.accept().await;
        assert!(result.is_ok());
        let result = avalanche_block.reject().await;
        assert!(result.is_ok());
    }


}
