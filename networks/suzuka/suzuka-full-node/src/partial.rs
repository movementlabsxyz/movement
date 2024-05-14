use std::{sync::Arc, time::Duration};

use anyhow::Context;
use suzuka_executor::{
    SuzukaExecutor,
    ExecutableBlock,
    HashValue,
    FinalityMode,
    Transaction,
    SignatureVerifiedTransaction,
    SignedTransaction,
    ExecutableTransactions,
    v1::SuzukaExecutorV1,
};
use m1_da_light_node_client::*;
use async_channel::{Sender, Receiver};
use sha2::Digest;
use crate::*;
use tokio_stream::StreamExt;
use tokio::sync::mpsc::{self, error::TrySendError};
use tokio::sync::RwLock;
use movement_types::{Block, BlockCommitment};
use mcr_settlement_client::{McrSettlementClient, McrSettlementClientOperations};

#[derive(Clone)]
pub struct SuzukaPartialNode<T, C>
{
    executor: T,
    transaction_sender : Sender<SignedTransaction>,
    pub transaction_receiver : Receiver<SignedTransaction>,
    light_node_client: Arc<RwLock<LightNodeServiceClient<tonic::transport::Channel>>>,
    settlement_client: Arc<C>,
}

impl<T, C> SuzukaPartialNode<T, C>
where
    T: SuzukaExecutor + Send + Sync,
    C: McrSettlementClientOperations + Send + Sync + 'static,
{

    pub fn new(
        executor: T,
        light_node_client: LightNodeServiceClient<tonic::transport::Channel>,
        settlement_client: C,
    ) -> Self {
        let (transaction_sender, transaction_receiver) = async_channel::unbounded();
        Self {
            executor : executor,
            transaction_sender,
            transaction_receiver,
            light_node_client : Arc::new(RwLock::new(light_node_client)),
            settlement_client: Arc::new(settlement_client)
        }
    }

	fn bind_transaction_channel(&mut self) {
		self.executor.set_tx_channel(self.transaction_sender.clone());
	}

	pub fn bound(
		executor: T,
		light_node_client: LightNodeServiceClient<tonic::transport::Channel>,
        settlement_client: C,
	) -> Result<Self, anyhow::Error> {
		let mut node = Self::new(executor, light_node_client, settlement_client);
		node.bind_transaction_channel();
		Ok(node)
    }

    pub async fn tick_write_transactions_to_da(&self) -> Result<(), anyhow::Error> {
        
        // limit the total time batching transactions
        let start_time = std::time::Instant::now();
        let end_time = start_time + std::time::Duration::from_millis(100);
        
        let mut transactions = Vec::new();


        while let Ok(transaction_result) = tokio::time::timeout(Duration::from_millis(100), self.transaction_receiver.recv()).await {

            match transaction_result {
                Ok(transaction) => {
                    println!("Got transaction: {:?}", transaction);
                    let serialized_transaction = serde_json::to_vec(&transaction)?;
                    transactions.push(BlobWrite {
                        data: serialized_transaction
                    });
                },
                Err(_) => {
                    break;
                }
            }

            if std::time::Instant::now() > end_time {
                break;
            }
        }

        if transactions.len() > 0 {
            let client_ptr = self.light_node_client.clone();
            let mut light_node_client = client_ptr.write().await;
            light_node_client.batch_write(
                BatchWriteRequest {
                    blobs: transactions
                }
            ).await?;
            println!("Wrote transactions to DA");
        }

        Ok(())


    }

    pub async fn write_transactions_to_da(&self) -> Result<(), anyhow::Error> {
        
        loop {
            self.tick_write_transactions_to_da().await?;
        }

    }

    // receive transactions from the transaction channel and send them to be executed
    // ! This assumes the m1 da light node is running sequencer mode
    pub async fn read_blocks_from_da(&self) -> Result<(), anyhow::Error> {

        let block_head_height = self.executor.get_block_head_height().await?;

        let (sender, receiver) = mpsc::channel(16);

        let mut stream = {
            let client_ptr = self.light_node_client.clone();
            let mut light_node_client =  client_ptr.write().await;
            light_node_client.stream_read_from_height(
                StreamReadFromHeightRequest {
                    height: block_head_height,
                }
            ).await?
        }.into_inner();

        let settlement_client = self.settlement_client.clone();
        // TODO: consume the commitment stream to finalize blocks
        let commitment_stream = settlement_client.stream_block_commitments().await?;

        tokio::spawn(async move {
            process_commitments(receiver, settlement_client).await;
        });

        while let Some(blob) = stream.next().await {

            println!("Stream hot!");
            // get the block
            let block_bytes = match blob?.blob.ok_or(anyhow::anyhow!("No blob in response"))?.blob_type.ok_or(anyhow::anyhow!("No blob type in response"))? {
                blob_response::BlobType::SequencedBlobBlock(blob) => {
                    blob.data
                },
                _ => { anyhow::bail!("Invalid blob type in response") }
            };

            // get the block
            let block : Block = serde_json::from_slice(&block_bytes)?;
            println!("Received block: {:?}", block);

            // get the transactions
            let mut block_transactions = Vec::new();
            for transaction in block.transactions {
                let signed_transaction : SignedTransaction = serde_json::from_slice(&transaction.0)?;
                let signature_verified_transaction = SignatureVerifiedTransaction::Valid(
                    Transaction::UserTransaction(
                        signed_transaction
                    )
                );
                block_transactions.push(signature_verified_transaction);
            }

            // form the executable transactions vec
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
            let block_id = executable_block.block_id;
            let commitment = self.executor.execute_block(
                FinalityMode::Opt,
                executable_block
            ).await?;

            println!("Executed block: {:?}", block_id);

            match sender.try_send(commitment) {
                Ok(_) => {},
                Err(TrySendError::Closed(_)) => {
                    break;
                },
                Err(TrySendError::Full(_commitment)) => {
                    println!("Commitment channel full, dropping commitment");
                }
            }
        }

        Ok(())

    }

}

async fn process_commitments<C>(
    mut receiver: mpsc::Receiver<BlockCommitment>,
    settlement_client: Arc<C>,
) -> Result<(), anyhow::Error>
where
    C: McrSettlementClientOperations,
{
    while let Some(commitment) = receiver.recv().await {
        println!("Got commitment: {:?}", commitment);
        settlement_client.post_block_commitment(commitment).await?;
    }
    Ok(())
}

impl<T, C> SuzukaFullNode for SuzukaPartialNode<T, C>
where
    T: SuzukaExecutor + Send + Sync,
    C: McrSettlementClientOperations + Send + Sync + 'static,
{
    
        /// Runs the services until crash or shutdown.
        async fn run_services(&self) -> Result<(), anyhow::Error> {

            self.executor.run_service().await?;

            Ok(())

        }

        /// Runs the background tasks until crash or shutdown.
        async fn run_background_tasks(&self) -> Result<(), anyhow::Error> {

            self.executor.run_background_tasks().await?;

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

impl SuzukaPartialNode<SuzukaExecutorV1, McrSettlementClient> {

    pub async fn try_from_env() -> Result<Self, anyhow::Error> {
        let (tx, _) = async_channel::unbounded();
        let light_node_client = LightNodeServiceClient::connect("http://[::1]:30730").await?;
        let executor = SuzukaExecutorV1::try_from_env(tx).await.context(
            "Failed to get executor from environment"
        )?;
        let settlement_client = McrSettlementClient::new();
        Self::bound(executor, light_node_client, settlement_client)
    }

}