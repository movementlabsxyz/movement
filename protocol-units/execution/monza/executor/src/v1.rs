use crate::*;
use monza_opt_executor::Executor;
use std::sync::Arc;
use tokio::sync::RwLock;
use async_channel::Sender;
use aptos_types::transaction::SignedTransaction;

#[derive(Clone)]
pub struct MonzaExecutorV1 {
    // this rwlock may be somewhat redundant
    pub executor: Executor,
    pub transaction_channel: Sender<SignedTransaction>,
}

impl MonzaExecutorV1 {
    pub fn new(executor : Executor, transaction_channel: Sender<SignedTransaction>) -> Self {
        Self {
            executor,
            transaction_channel,
        }
    }

    pub async fn try_from_env(transaction_channel : Sender<SignedTransaction>) -> Result<Self, anyhow::Error> {
        let executor = Executor::try_from_env()?;
        Ok(Self::new(executor, transaction_channel))
    }

}

#[tonic::async_trait]
impl MonzaExecutor for MonzaExecutorV1 {

    /// Runs the service.
    async fn run_service(&self) -> Result<(), anyhow::Error> {
        self.executor.run_service().await
    }

    /// Runs the necessary background tasks.
    async fn run_background_tasks(&self) -> Result<(), anyhow::Error> {
        
        loop {
            // readers should be able to run concurrently
            self.executor.tick_transaction_pipe(self.transaction_channel.clone()).await?;
        }

        Ok(())

    }
    
    /// Executes a block dynamically
    async fn execute_block(
        &self,
        mode : &FinalityMode, 
        block: ExecutableBlock,
    ) -> Result<StateCheckpointOutput, anyhow::Error> {

        match mode {
            FinalityMode::Dyn => unimplemented!(),
            FinalityMode::Opt => {
                self.executor.execute_block(block).await
            },
            FinalityMode::Fin => unimplemented!(),
        }

    }

    /// Sets the transaction channel.
    async fn set_tx_channel(&mut self, tx_channel: Sender<SignedTransaction>) -> Result<(), anyhow::Error> {
        self.transaction_channel = tx_channel;
        Ok(())
    }

    /// Gets the API.
    async fn get_api(
        &self,
        _mode : &FinalityMode, 
    ) -> Result<Apis, anyhow::Error> {
        match _mode {
            FinalityMode::Dyn => unimplemented!(),
            FinalityMode::Opt => {
                Ok(self.executor.try_get_apis().await?)
            },
            FinalityMode::Fin => unimplemented!(),
        }
    }

    /// Get block head height.
    async fn get_block_head_height(&self) -> Result<u64, anyhow::Error> {
        unimplemented!()
    }

}

#[cfg(test)]
mod opt_tests {

	use super::*;
	use aptos_crypto::{
		ed25519::{Ed25519PrivateKey, Ed25519Signature},
		HashValue, PrivateKey, Uniform,
	};
	use aptos_types::{
		account_address::AccountAddress,
		block_executor::partitioner::ExecutableTransactions,
		chain_id::ChainId,
		transaction::{
			signature_verified_transaction::SignatureVerifiedTransaction, RawTransaction, Script,
			SignedTransaction, Transaction, TransactionPayload
		}
	};
	use aptos_api::{
		accept_type::AcceptType,
		transactions::SubmitTransactionPost
	};
    use futures::SinkExt;
    use aptos_mempool::{
        MempoolClientRequest, MempoolClientSender,
    };
    use futures::channel::oneshot;

	fn create_signed_transaction(gas_unit_price: u64) -> SignedTransaction {
		let private_key = Ed25519PrivateKey::generate_for_testing();
		let public_key = private_key.public_key();
		let transaction_payload = TransactionPayload::Script(Script::new(vec![0], vec![], vec![]));
		let raw_transaction = RawTransaction::new(
			AccountAddress::random(),
			0,
			transaction_payload,
			0,
			gas_unit_price,
			0,
			ChainId::new(10), // This is the value used in aptos testing code.
		);
		SignedTransaction::new(raw_transaction, public_key, Ed25519Signature::dummy_signature())
	}


	#[tokio::test]
	async fn test_execute_opt_block() -> Result<(), anyhow::Error> {
        let (tx, rx) = async_channel::unbounded();
		let mut executor = MonzaExecutorV1::try_from_env(tx).await?;
		let block_id = HashValue::random();
		let tx = SignatureVerifiedTransaction::Valid(Transaction::UserTransaction(
			create_signed_transaction(0),
		));
		let txs = ExecutableTransactions::Unsharded(vec![tx]);
		let block = ExecutableBlock::new(block_id.clone(), txs);
		executor.execute_block(&FinalityMode::Opt, block).await?;
		Ok(())
	}


	#[tokio::test]
	async fn test_pipe_transactions_from_api() -> Result<(), anyhow::Error> {

        let (tx, rx) = async_channel::unbounded();
		let executor = MonzaExecutorV1::try_from_env(tx).await?;
		let services_executor = executor.clone();
        let background_executor = executor.clone();

        let services_handle = tokio::spawn(async move {
            services_executor.run_service().await?;
            Ok(()) as Result<(), anyhow::Error>
        });

        let background_handle = tokio::spawn(async move {
            background_executor.run_background_tasks().await?;
            Ok(()) as Result<(), anyhow::Error>
        });

        // Start the background tasks
		let user_transaction = create_signed_transaction(0);
		let comparison_user_transaction = user_transaction.clone();
		let bcs_user_transaction = bcs::to_bytes(&user_transaction)?;


		let request = SubmitTransactionPost::Bcs(
			aptos_api::bcs_payload::Bcs(bcs_user_transaction)
		);
		let api = executor.get_api(&FinalityMode::Opt).await?;
		api.transactions.submit_transaction(AcceptType::Bcs, request).await?;

		services_handle.abort();
        background_handle.abort();
		let received_transaction = rx.recv().await?;
		assert_eq!(received_transaction, comparison_user_transaction);
	
		Ok(())
	}

    #[tokio::test]
	async fn test_pipe_transactions_from_api_and_execute() -> Result<(), anyhow::Error> {

        let (tx, rx) = async_channel::unbounded();
		let executor = MonzaExecutorV1::try_from_env(tx).await?;
		let services_executor = executor.clone();
        let background_executor = executor.clone();

        let services_handle = tokio::spawn(async move {
            services_executor.run_service().await?;
            Ok(()) as Result<(), anyhow::Error>
        });

        let background_handle = tokio::spawn(async move {
            background_executor.run_background_tasks().await?;
            Ok(()) as Result<(), anyhow::Error>
        });

        // Start the background tasks
		let user_transaction = create_signed_transaction(0);
		let comparison_user_transaction = user_transaction.clone();
		let bcs_user_transaction = bcs::to_bytes(&user_transaction)?;


		let request = SubmitTransactionPost::Bcs(
			aptos_api::bcs_payload::Bcs(bcs_user_transaction)
		);
		let api = executor.get_api(&FinalityMode::Opt).await?;
		api.transactions.submit_transaction(AcceptType::Bcs, request).await?;

		let received_transaction = rx.recv().await?;
		assert_eq!(received_transaction, comparison_user_transaction);

        // Now execute the block
        let block_id = HashValue::random();
        let tx = SignatureVerifiedTransaction::Valid(Transaction::UserTransaction(
            received_transaction
        ));
        let txs = ExecutableTransactions::Unsharded(vec![tx]);
        let block = ExecutableBlock::new(block_id.clone(), txs);
        executor.execute_block(&FinalityMode::Opt, block).await?;

        services_handle.abort();
        background_handle.abort();

		Ok(())
	}

}
