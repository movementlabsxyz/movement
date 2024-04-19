use std::sync::Arc;

use monza_executor::{
    MonzaExecutor,
    SignatureVerifiedTransaction,
    ExecutableBlock,
    HashValue,
    // v1::MonzaExecutorV1,
};
use m1_da_light_node_client::{LightNodeServiceClient, StreamReadFromHeightRequest, BatchWriteRequest};
use async_channel::{Sender, Receiver};
use sha2::Digest;
use crate::*;

pub struct MonzaPartialFullNode<T : MonzaExecutor + Send + Sync> {
    executor: Box<dyn MonzaExecutor>,
    transaction_sender : Sender<SignatureVerifiedTransaction>,
    transaction_reeiver : Receiver<SignatureVerifiedTransaction>,
    light_node_client: LightNodeServiceClient<tonic::transport::Channel>,
}

impl <T : MonzaExecutor + Send + Sync>MonzaPartialFullNode<T> {

    pub fn new(executor : T, light_node_client: LightNodeServiceClient<tonic::transport::Channel>) -> Self {
        let (transaction_sender, transaction_reeiver) = async_channel::unbounded();
        Self {
            executor : Box::new(executor),
            transaction_sender,
            transaction_reeiver,
            light_node_client,
        }
    }

    pub fn bind_transaction_channel(&self) {
        self.executor.set_tx_channel(self.transaction_sender.clone());
    }

    pub fn bound(executor : T, light_node_client: LightNodeServiceClient<tonic::transport::Channel>) -> Self {
        let node = Self::new(executor, light_node_client);
        node.bind_transaction_channel();
        node
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

            let me = Arc::new(self.clone());

            // receive transactions from the transaction channel and send them to the da
            // ! This assumes the m1 da light node is running sequencer mode
            let da_writer_task = tokio::spawn(async move {
                while let Ok(transaction) = self.transaction_reeiver.recv().await {
                    let serialized_transaction = serde_json::to_vec(&transaction)?;
                    me.light_node_client.batch_write(
                        BatchWriteRequest {
                            blobs: vec![serialized_transaction],
                        }
                    ).await?;
                }
                Ok(())
            });

            // assume the opt state is close to synced, just get the block head height reported and read from there
            let da_reader_task = tokio::spawn(async move {
                let block_head_height = me.executor.get_block_head_height().await?;
                
                let stream = self.light_node_client.stream_read_from_height(
                    StreamReadFromHeightRequest {
                        height: block_head_height,
                    }
                ).await?.into_inner();

                while let Some(blob) = stream.recv().await? {

                    // get the block
                    let block_bytes = blob.into_inner().blobs;
                    let block : Vec<SignatureVerifiedTransaction> = serde_json::from_slice(&block_bytes)?;
                    
                    // hash the block bytes
                    let mut hasher = sha2::Sha256::new();
                    hasher.update(&block_bytes);
                    let slice = hasher.finalize().to_vec();
                    let block_hash = HashValue::try_from(slice)?;

                    // form the executable block and execute it
                    let executable_block = ExecutableBlock::new(
                        block_hash,
                        block
                    );
                    me.executor.execute_opt_block(block).await?;

                }

                Ok(())
        
            });

            // wait for both tasks to finish
            tokio::try_join!(da_writer_task, da_reader_task)?;

            Ok(())
        

      }

}