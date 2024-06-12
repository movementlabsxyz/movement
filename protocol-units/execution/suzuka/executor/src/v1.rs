// FIXME: glob imports are bad style
use crate::*;
use aptos_types::transaction::SignedTransaction;
use maptos_opt_executor::Executor;
use movement_types::BlockCommitment;

use async_channel::Sender;
use tracing::debug;

#[derive(Clone)]
pub struct SuzukaExecutorV1 {
	// this rwlock may be somewhat redundant
	pub executor: Executor,
	pub transaction_channel: Sender<SignedTransaction>,
}

impl SuzukaExecutorV1 {
	pub fn new(executor: Executor, transaction_channel: Sender<SignedTransaction>) -> Self {
		Self { executor, transaction_channel }
	}

	pub async fn try_from_env(
		transaction_channel: Sender<SignedTransaction>,
	) -> Result<Self, anyhow::Error> {
		let executor = Executor::try_from_env()?;
		Ok(Self::new(executor, transaction_channel))
	}

	pub async fn try_from_config(
		transaction_channel: Sender<SignedTransaction>,
		config: maptos_execution_util::config::Config,
	) -> Result<Self, anyhow::Error> {
		let executor = Executor::try_from_config(config).await?;
		Ok(Self::new(executor, transaction_channel))
	}

}

#[tonic::async_trait]
impl SuzukaExecutor for SuzukaExecutorV1 {
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

	async fn execute_block_opt(
		&self,
		block: ExecutableBlock,
	) -> Result<BlockCommitment, anyhow::Error> {
		debug!("Executing block: {:?}", block.block_id);
		self.executor.execute_block(block).await
	}

	/// Sets the transaction channel.
	fn set_tx_channel(&mut self, tx_channel: Sender<SignedTransaction>) {
		self.transaction_channel = tx_channel;
	}

	/// Gets the API.
	fn get_apis(&self) -> Apis {
		self.executor.get_apis()
	}

	/// Get block head height.
	async fn get_block_head_height(&self) -> Result<u64, anyhow::Error> {
		self.executor.get_block_head_height()
	}

	/// Build block metadata for a timestamp
	async fn build_block_metadata(
		&self,
		block_id: HashValue,
		timestamp: u64,
	) -> Result<BlockMetadata, anyhow::Error> {
		let (epoch, round) = self.executor.get_next_epoch_and_round().await?;
		// Clone the signer from the executor for signing the metadata.
		let signer = self.executor.signer.clone();

		// Create a block metadata transaction.
		Ok(BlockMetadata::new(block_id, epoch, round, signer.author(), vec![], vec![], timestamp))
	}
}

#[cfg(test)]
mod opt_tests {

	use super::*;
	use aptos_api::{accept_type::AcceptType, transactions::SubmitTransactionPost};
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
			SignedTransaction, Transaction, TransactionPayload,
		},
	};

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
			ChainId::test(), // This is the value used in aptos testing code.
		);
		SignedTransaction::new(raw_transaction, public_key, Ed25519Signature::dummy_signature())
	}

	#[tokio::test]
	async fn test_execute_opt_block() -> Result<(), anyhow::Error> {
		let (tx, _rx) = async_channel::unbounded();
		let executor = SuzukaExecutorV1::try_from_env(tx).await?;
		let block_id = HashValue::random();
		let tx = SignatureVerifiedTransaction::Valid(Transaction::UserTransaction(
			create_signed_transaction(0),
		));
		let txs = ExecutableTransactions::Unsharded(vec![tx]);
		let block = ExecutableBlock::new(block_id.clone(), txs);
		executor.execute_block_opt(block).await?;
		Ok(())
	}

	#[tokio::test]
	async fn test_pipe_transactions_from_api() -> Result<(), anyhow::Error> {
		let (tx, rx) = async_channel::unbounded();
		let executor = SuzukaExecutorV1::try_from_env(tx).await?;
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

		let request = SubmitTransactionPost::Bcs(aptos_api::bcs_payload::Bcs(bcs_user_transaction));
		let api = executor.get_apis();
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
		let executor = SuzukaExecutorV1::try_from_env(tx).await?;
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

		let request = SubmitTransactionPost::Bcs(aptos_api::bcs_payload::Bcs(bcs_user_transaction));
		let api = executor.get_apis();
		api.transactions.submit_transaction(AcceptType::Bcs, request).await?;

		let received_transaction = rx.recv().await?;
		assert_eq!(received_transaction, comparison_user_transaction);

		// Now execute the block
		let block_id = HashValue::random();
		let block_metadata = executor
			.build_block_metadata(block_id.clone(), chrono::Utc::now().timestamp_micros() as u64)
			.await
			.unwrap();
		let txs = ExecutableTransactions::Unsharded(
			[
				Transaction::BlockMetadata(block_metadata),
				Transaction::UserTransaction(received_transaction),
			]
			.into_iter()
			.map(SignatureVerifiedTransaction::Valid)
			.collect(),
		);
		let block = ExecutableBlock::new(block_id.clone(), txs);
		let commitment = executor.execute_block_opt(block).await?;

		assert_eq!(commitment.block_id.to_vec(), block_id.to_vec());
		assert_eq!(commitment.height, 1);

		services_handle.abort();
		background_handle.abort();

		Ok(())
	}
}
