mod services;
pub mod v1;

use maptos_opt_executor::executor::ExecutionState;
use maptos_opt_executor::executor::TxExecutionResult;
use services::Services;
use tokio::sync::mpsc::UnboundedReceiver;

pub use aptos_crypto::hash::HashValue;
pub use aptos_types::{
	block_executor::partitioner::ExecutableBlock,
	block_executor::partitioner::ExecutableTransactions,
	block_metadata::BlockMetadata,
	transaction::signature_verified_transaction::SignatureVerifiedTransaction,
	transaction::{SignedTransaction, Transaction},
};
use async_trait::async_trait;
use maptos_execution_util::config::Config;
use movement_types::block::BlockCommitment;
use std::future::Future;

#[async_trait]
pub trait DynOptFinExecutor {
	type Context: MakeOptFinServices;

	/// Initialize the background task responsible for transaction processing.
	fn background(
		&self,
		mempool_commit_tx_receiver: UnboundedReceiver<Vec<TxExecutionResult>>,
		config: &Config,
	) -> Result<
		(Self::Context, impl Future<Output = Result<(), anyhow::Error>> + Send + 'static),
		anyhow::Error,
	>;

	/// Checks whether the transaction had already been executed by opt
	fn has_executed_transaction_opt(
		&self,
		transaction_hash: HashValue,
	) -> Result<bool, anyhow::Error>;

	/// Executes a block optimistically
	fn execute_block_opt(
		&mut self,
		block: ExecutableBlock,
	) -> Result<(BlockCommitment, ExecutionState), anyhow::Error>;

	/// Update the height of the latest finalized block
	fn set_finalized_block_height(&self, block_height: u64) -> Result<(), anyhow::Error>;

	/// Gets the block commitment for a given height
	async fn get_commitment_for_height(
		&self,
		block_height: u64,
	) -> Result<BlockCommitment, anyhow::Error>;

	/// Gets the block commitment for a given version.
	async fn get_block_commitment_by_version(
		&self,
		block_height: u64,
	) -> Result<BlockCommitment, anyhow::Error>;

	/// Revert the chain to the specified height
	async fn revert_block_head_to(&self, block_height: u64) -> Result<(), anyhow::Error>;

	/// Get block head height.
	fn get_block_head_height(&self) -> Result<u64, anyhow::Error>;

	/// Build block metadata for a timestamp
	fn build_block_metadata(
		&self,
		block_id: HashValue,
		timestamp: u64,
	) -> Result<BlockMetadata, anyhow::Error>;

	/// Decrements transactions in flight on the transaction channel.
	fn decrement_transactions_in_flight(&self, count: u64);

	/// Gets the config
	fn config(&self) -> &Config;
}

pub trait MakeOptFinServices {
	fn services(&self) -> Services;
}

#[cfg(test)]
mod tests {
	use crate::{v1::Executor, DynOptFinExecutor};
	use crate::{ExecutableBlock, ExecutableTransactions, SignatureVerifiedTransaction};
	use anyhow::Context;
	use aptos_crypto::{ed25519::Ed25519PrivateKey, HashValue, Uniform};
	use aptos_types::account_address::AccountAddress;
	use aptos_types::chain_id::ChainId;
	use aptos_types::transaction::{
		RawTransaction, Script, SignedTransaction, Transaction, TransactionPayload,
	};
	use maptos_execution_util::config::Config;
	use maptos_opt_executor::executor::TxExecutionResult;
	use movement_signer_loader::identifiers::{local::Local, SignerIdentifier};
	use movement_signer_test::ed25519::TestSigner;
	use movement_signing_aptos::TransactionSigner;
	use tempfile::TempDir;
	use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver};

	async fn setup(
		mut maptos_config: Config,
	) -> Result<(Executor, TempDir, UnboundedReceiver<Vec<TxExecutionResult>>), anyhow::Error> {
		let tempdir = tempfile::tempdir()?;
		// replace the db path with the temporary directory
		maptos_config.chain.maptos_db_path.replace(tempdir.path().to_path_buf());
		// No mempool don't use the receiver.
		let (mempool_tx_exec_result_sender, mempool_commit_tx_receiver) =
			unbounded_channel::<Vec<TxExecutionResult>>();
		let executor =
			Executor::try_from_config(maptos_config, mempool_tx_exec_result_sender).await?;
		Ok((executor, tempdir, mempool_commit_tx_receiver))
	}

	async fn create_signed_transaction(
		signer: &impl TransactionSigner,
	) -> Result<SignedTransaction, anyhow::Error> {
		let transaction_payload = TransactionPayload::Script(Script::new(vec![0], vec![], vec![]));
		let raw_transaction = RawTransaction::new(
			AccountAddress::random(),
			0,
			transaction_payload,
			0,
			0,
			0,
			ChainId::test(), // This is the value used in aptos testing code.
		);
		signer.sign_transaction(raw_transaction).await.context("failed to sign")
	}

	#[tokio::test]
	async fn execute_signed_transaction() -> Result<(), anyhow::Error> {
		let private_key = Ed25519PrivateKey::generate_for_testing();
		let mut config = Config::default();
		let signing_key = ed25519_dalek::SigningKey::from_bytes(&private_key.to_bytes());
		let private_key_hex_bytes = hex::encode(&private_key.to_bytes());
		config.chain.maptos_private_key_signer_identifier =
			SignerIdentifier::Local(Local { private_key_hex_bytes });
		let signer = TestSigner::new(signing_key);
		let (mut executor, _tempdir, _mempool_commit_tx_receiver) = setup(config).await?;
		let transaction = create_signed_transaction(&signer).await?;
		let block_id = HashValue::random();
		let block_metadata = executor
			.build_block_metadata(block_id.clone(), chrono::Utc::now().timestamp_micros() as u64)
			.unwrap();
		let txs = ExecutableTransactions::Unsharded(
			[Transaction::BlockMetadata(block_metadata), Transaction::UserTransaction(transaction)]
				.into_iter()
				.map(SignatureVerifiedTransaction::Valid)
				.collect(),
		);
		let block = ExecutableBlock::new(block_id.clone(), txs);
		executor.execute_block_opt(block)?;
		Ok(())
	}
}
