use std::sync::Arc;

use monza_executor::{
    MonzaExecutor,
    SignatureVerifiedTransaction,
    ExecutableBlock,
    HashValue,
    FinalityMode,
    ExecutableTransactions
    // v1::MonzaExecutorV1,
};
use m1_da_light_node_client::{LightNodeServiceClient, StreamReadFromHeightRequest, BatchWriteRequest, BlobWrite};
use async_channel::{Sender, Receiver};
use sha2::Digest;
use crate::*;
use tokio_stream::StreamExt;
use tokio::sync::RwLock;

pub struct MonzaPartialFullNode<T : MonzaExecutor + Send + Sync> {
    executor: T,
    transaction_sender : Sender<SignatureVerifiedTransaction>,
    transaction_reeiver : Receiver<SignatureVerifiedTransaction>,
    light_node_client: Arc<RwLock<LightNodeServiceClient<tonic::transport::Channel>>>
}

impl <T : MonzaExecutor + Send + Sync>MonzaPartialFullNode<T> {

    pub fn new(executor : T, light_node_client: LightNodeServiceClient<tonic::transport::Channel>) -> Self {
        let (transaction_sender, transaction_reeiver) = async_channel::unbounded();
        Self {
            executor : executor,
            transaction_sender,
            transaction_reeiver,
            light_node_client : Arc::new(RwLock::new(light_node_client))
        }
    }

    pub async fn bind_transaction_channel(&self) -> Result<(), anyhow::Error> {
        self.executor.set_tx_channel(self.transaction_sender.clone()).await?;
        Ok(())
    }

    pub async fn bound(executor : T, light_node_client: LightNodeServiceClient<tonic::transport::Channel>) -> Result<Self, anyhow::Error> {
        let node = Self::new(executor, light_node_client);
        node.bind_transaction_channel().await?;
        Ok(node)
    }

    pub async fn write_transactions_to_da(&self) -> Result<(), anyhow::Error> {
        
        while let Ok(transaction) = self.transaction_reeiver.recv().await {
            let serialized_transaction = serde_json::to_vec(&transaction)?;
            {
                let client_ptr = self.light_node_client.clone();
                let mut light_node_client = client_ptr.write().await;
                light_node_client.batch_write(
                    BatchWriteRequest {
                        blobs: vec![BlobWrite {
                            data: serialized_transaction,
                        }],
                    }
                ).await?;
            }
        }

        Ok(())


    }

    // receive transactions from the transaction channel and send them to the da
    // ! This assumes the m1 da light node is running sequencer mode
    pub async fn read_blocks_from_da(&self) -> Result<(), anyhow::Error> {
        
        let block_head_height = self.executor.get_block_head_height().await?;
        
        let mut stream = {
            let client_ptr = self.light_node_client.clone();
            let mut light_node_client =  client_ptr.write().await;
            light_node_client.stream_read_from_height(
                StreamReadFromHeightRequest {
                    height: block_head_height,
                }
            ).await?
        }.into_inner();

        while let Some(blob) = stream.next().await {
            // get the block
            let block_bytes = blob?.blob.ok_or(anyhow::anyhow!("No blob in response"))?.data;
            let block_transactions : Vec<SignatureVerifiedTransaction> = serde_json::from_slice(&block_bytes)?;
            let block = ExecutableTransactions::Unsharded(
                block_transactions
            );
            
            // hash the block bytes
            let mut hasher = sha2::Sha256::new();
            hasher.update(&block_bytes);
            let slice = hasher.finalize();
            let block_hash = HashValue::from_slice(slice.as_slice())?;

            // form the executable block and execute it
            let executable_block = ExecutableBlock::new(
                block_hash,
                block
            );
            self.executor.execute_block(
                &FinalityMode::Opt,
                executable_block
            ).await?;

        }

        Ok(())

    }

}

impl <T : MonzaExecutor + Send + Sync>MonzaFullNode for MonzaPartialFullNode<T> {
    
      /// Runs the services until crash or shutdown.
      async fn run_services(&self) -> Result<(), anyhow::Error> {

            // get each of the apis

            // synthesize a new api over them using some abstraction

            // start the open api service over it

            Ok(())

      }

      // ! Currently this only implements opt.
      /// Runs the executor until crash or shutdown.
      async fn run_executor(&self) -> Result<(), anyhow::Error> {

            // wait for both tasks to finish
            tokio::try_join!(
                self.write_transactions_to_da(),
                self.read_blocks_from_da()
            )?;

            Ok(())
        

      }

}