use crate::{
	BlockMetadata, DynOptFinExecutor, ExecutableBlock, HashValue, MakeOptFinServices, Services,
	SignedTransaction,
};
use maptos_execution_util::config::Config;
use maptos_fin_view::FinalityView;
use maptos_opt_executor::{Context as OptContext, Executor as OptExecutor};
use movement_types::block::BlockCommitment;

use anyhow::format_err;
use async_trait::async_trait;
use tokio::sync::mpsc::Sender;
use tracing::debug;

use std::future::Future;

pub struct Executor {
	executor: OptExecutor,
	finality_view: FinalityView,
}

pub struct Context {
	opt_context: OptContext,
	fin_service: maptos_fin_view::Service,
}

impl Executor {
	/// Creates the execution state with the optimistic executor
	/// and the finality view, joined at the hip by shared storage.
	pub fn new(executor: OptExecutor) -> Self {
		let finality_view = FinalityView::new(executor.db_reader());
		Self { executor, finality_view }
	}

	pub fn try_from_config(config: Config) -> Result<Self, anyhow::Error> {
		let executor = OptExecutor::try_from_config(config)?;
		Ok(Self::new(executor))
	}
}

impl MakeOptFinServices for Context {
	fn services(&self) -> Services {
		let opt = maptos_opt_executor::Service::new(&self.opt_context);
		let fin = self.fin_service.clone();
		Services::new(opt, fin)
	}
}

#[async_trait]
impl DynOptFinExecutor for Executor {
	type Context = Context;

	fn background(
		&self,
		transaction_sender: Sender<SignedTransaction>,
	) -> Result<
		(Context, impl Future<Output = Result<(), anyhow::Error>> + Send + 'static),
		anyhow::Error,
	> {
		let (opt_context, background) = self.executor.background(transaction_sender)?;
		let fin_service = self.finality_view.service(
			opt_context.mempool_client_sender(),
			self.config(),
			opt_context.node_config().clone(),
		);
		let indexer_runtime = opt_context.run_indexer_grpc_service()?;
		let background = async move {
			// The indexer runtime should live as long as the Tx pipe.
			let _indexer_runtime = indexer_runtime;
			background.run().await?;
			Ok(())
		};
		Ok((Context { opt_context, fin_service }, background))
	}

	fn has_executed_transaction_opt(
		&self,
		transaction_hash: HashValue,
	) -> Result<bool, anyhow::Error> {
		self.executor.has_executed_transaction(transaction_hash)
	}

	async fn execute_block_opt(
		&self,
		block: ExecutableBlock,
	) -> Result<BlockCommitment, anyhow::Error> {
		debug!("Executing block: {:?}", block.block_id);
		self.executor.execute_block(block).await
	}

	fn set_finalized_block_height(&self, height: u64) -> Result<(), anyhow::Error> {
		self.finality_view.set_finalized_block_height(height)
	}

	async fn revert_block_head_to(&self, block_height: u64) -> Result<(), anyhow::Error> {
		if let Some(final_height) = self.finality_view.finalized_block_height() {
			if block_height < final_height {
				return Err(format_err!(
					"Can't revert to height {block_height} preciding the finalized height {final_height}"
				));
			}
		}
		self.executor.revert_block_head_to(block_height).await
	}

	/// Get block head height.
	fn get_block_head_height(&self) -> Result<u64, anyhow::Error> {
		self.executor.get_block_head_height()
	}

	/// Build block metadata for a timestamp
	fn build_block_metadata(
		&self,
		block_id: HashValue,
		timestamp: u64,
	) -> Result<BlockMetadata, anyhow::Error> {
		let (epoch, round) = self.executor.get_next_epoch_and_round()?;
		let signer = &self.executor.signer;

		// Create a block metadata transaction.
		Ok(BlockMetadata::new(block_id, epoch, round, signer.author(), vec![], vec![], timestamp))
	}

	/// Rollover the genesis block
	async fn rollover_genesis_block(&self) -> Result<(), anyhow::Error> {
		self.executor.rollover_genesis_now().await
	}

	fn decrement_transactions_in_flight(&self, count: u64) {
		self.executor.decrement_transactions_in_flight(count)
	}

	fn config(&self) -> &Config {
		self.executor.config()
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use aptos_api::{accept_type::AcceptType, transactions::SubmitTransactionPost};
	use aptos_crypto::{
		ed25519::{Ed25519PrivateKey, Ed25519Signature},
		HashValue, PrivateKey, Uniform,
	};
	use aptos_sdk::{
		bcs,
		transaction_builder::TransactionFactory,
		types::{AccountKey, LocalAccount},
	};
	use aptos_types::{
		account_address::AccountAddress,
		account_config::aptos_test_root_address,
		block_executor::partitioner::ExecutableTransactions,
		chain_id::ChainId,
		transaction::{
			signature_verified_transaction::SignatureVerifiedTransaction, RawTransaction, Script,
			SignedTransaction, Transaction, TransactionPayload, Version,
		},
	};
	use maptos_execution_util::config::Config;

	use rand::SeedableRng;
	use tokio::sync::mpsc;

	use std::collections::HashMap;

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
		let config = Config::default();
		let executor = Executor::try_from_config(config)?;
		let block_id = HashValue::random();
		let block_metadata = executor
			.build_block_metadata(block_id.clone(), chrono::Utc::now().timestamp_micros() as u64)
			.unwrap();
		let txs = ExecutableTransactions::Unsharded(
			[
				Transaction::BlockMetadata(block_metadata),
				Transaction::UserTransaction(create_signed_transaction(0)),
			]
			.into_iter()
			.map(SignatureVerifiedTransaction::Valid)
			.collect(),
		);
		let block = ExecutableBlock::new(block_id.clone(), txs);
		executor.execute_block_opt(block).await?;
		Ok(())
	}

	#[tokio::test]
	async fn test_pipe_transactions_from_api() -> Result<(), anyhow::Error> {
		let config = Config::default();
		let (tx_sender, mut tx_receiver) = mpsc::channel(16);
		let executor = Executor::try_from_config(config)?;
		let (context, background) = executor.background(tx_sender)?;
		let services = context.services();
		let api = services.get_opt_apis();

		let services_handle = tokio::spawn(services.run());
		let background_handle = tokio::spawn(background);

		// Start the background tasks
		let user_transaction = create_signed_transaction(0);
		let comparison_user_transaction = user_transaction.clone();
		let bcs_user_transaction = bcs::to_bytes(&user_transaction)?;

		let request = SubmitTransactionPost::Bcs(aptos_api::bcs_payload::Bcs(bcs_user_transaction));
		api.transactions.submit_transaction(AcceptType::Bcs, request).await?;

		services_handle.abort();
		background_handle.abort();
		let received_transaction = tx_receiver.recv().await.unwrap();
		assert_eq!(received_transaction, comparison_user_transaction);

		Ok(())
	}

	#[tokio::test]
	async fn test_pipe_transactions_from_api_and_execute() -> Result<(), anyhow::Error> {
		let config = Config::default();
		let (tx_sender, mut tx_receiver) = mpsc::channel(16);
		let executor = Executor::try_from_config(config)?;
		let (context, background) = executor.background(tx_sender)?;
		let services = context.services();
		let api = services.get_opt_apis();

		let services_handle = tokio::spawn(services.run());
		let background_handle = tokio::spawn(background);

		// Start the background tasks
		let user_transaction = create_signed_transaction(0);
		let comparison_user_transaction = user_transaction.clone();
		let bcs_user_transaction = bcs::to_bytes(&user_transaction)?;

		let request = SubmitTransactionPost::Bcs(aptos_api::bcs_payload::Bcs(bcs_user_transaction));
		api.transactions.submit_transaction(AcceptType::Bcs, request).await?;

		let received_transaction = tx_receiver.recv().await.unwrap();
		assert_eq!(received_transaction, comparison_user_transaction);

		// Now execute the block
		let block_id = HashValue::random();
		let block_metadata = executor
			.build_block_metadata(block_id.clone(), chrono::Utc::now().timestamp_micros() as u64)
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

		assert_eq!(commitment.block_id().to_vec(), block_id.to_vec());
		assert_eq!(commitment.height(), 1);

		services_handle.abort();
		background_handle.abort();

		Ok(())
	}

	#[tokio::test]
	async fn test_revert_chain_state_at_nth_commit() -> Result<(), anyhow::Error> {
		use aptos_db::db::test_helper::arb_blocks_to_commit_with_block_nums;
		use aptos_proptest_helpers::ValueGenerator;

		#[derive(Debug)]
		struct Commit {
			current_version: Version,
		}

		let config = Config::default();
		let (tx_sender, mut tx_receiver) = mpsc::channel(16);
		let executor = Executor::try_from_config(config)?;
		let (context, background) = executor.background(tx_sender)?;
		let services = context.services();
		let api = services.get_opt_apis();

		let services_handle = tokio::spawn(services.run());
		let background_handle = tokio::spawn(background);

		let mut committed_blocks = HashMap::new();

		let mut val_generator = ValueGenerator::new();
		// set range of min and max blocks to 5 to always gen 5 blocks
		let (blocks, _) = val_generator.generate(arb_blocks_to_commit_with_block_nums(5, 5));
		let mut blockheight = 0;
		let mut current_version: Version = 0;
		let mut commit_versions = vec![];

		for (txns_to_commit, _ledger_info_with_sigs) in &blocks {
			let user_transaction = create_signed_transaction(0);
			let comparison_user_transaction = user_transaction.clone();
			let bcs_user_transaction = bcs::to_bytes(&user_transaction)?;

			let request =
				SubmitTransactionPost::Bcs(aptos_api::bcs_payload::Bcs(bcs_user_transaction));
			api.transactions.submit_transaction(AcceptType::Bcs, request).await?;

			let received_transaction = tx_receiver.recv().await.unwrap();
			assert_eq!(received_transaction, comparison_user_transaction);

			// Now execute the block
			let block_id = HashValue::random();
			let block_metadata = executor
				.build_block_metadata(
					block_id.clone(),
					chrono::Utc::now().timestamp_micros() as u64,
				)
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
			executor.execute_block_opt(block).await?;

			blockheight += 1;
			current_version += txns_to_commit.len() as u64;
			committed_blocks.insert(blockheight, Commit { current_version });
			commit_versions.push(current_version);
			//blockheight += 1;
		}

		// Get the 3rd block back from the latest block
		let revert_block_num = blockheight - 3;
		let revert = committed_blocks.get(&revert_block_num).unwrap();

		// Get the version to revert to
		let version_to_revert_to = revert.current_version;

		executor.revert_block_head_to(version_to_revert_to).await?;

		let latest_version = {
			let db_reader = executor.executor.db_reader().clone();
			db_reader.get_synced_version()?
		};
		assert_eq!(latest_version, version_to_revert_to);

		services_handle.abort();
		background_handle.abort();
		Ok(())
	}

	#[tokio::test]
	async fn test_execute_block_state_get_api() -> Result<(), anyhow::Error> {
		// Create an executor instance from the environment configuration.
		let config = Config::default();
		let (tx_sender, _tx_receiver) = mpsc::channel(16);
		let executor = Executor::try_from_config(config)?;
		let (context, background) = executor.background(tx_sender)?;
		let config = executor.config();
		let services = context.services();
		let apis = services.get_opt_apis();

		let background_handle = tokio::spawn(background);

		// Initialize a root account using a predefined keypair and the test root address.
		let root_account = LocalAccount::new(
			aptos_test_root_address(),
			AccountKey::from_private_key(config.chain.maptos_private_key.clone()),
			0,
		);

		// Seed for random number generator, used here to generate predictable results in a test environment.
		let seed = [3u8; 32];
		let mut rng = ::rand::rngs::StdRng::from_seed(seed);

		// Create a transaction factory with the chain ID of the executor.
		let tx_factory = TransactionFactory::new(config.chain.maptos_chain_id);

		// Simulate the execution of multiple blocks.
		for _ in 0..10 {
			// For example, create and execute 3 blocks.
			let block_id = HashValue::random(); // Generate a random block ID for each block.
			let block_metadata = executor
				.build_block_metadata(
					block_id.clone(),
					chrono::Utc::now().timestamp_micros() as u64,
				)
				.unwrap();

			// Generate new accounts and create transactions for each block.
			let mut transactions = Vec::new();
			let mut transaction_hashes = Vec::new();
			transactions.push(Transaction::BlockMetadata(block_metadata));
			for _ in 0..2 {
				// Each block will contain 2 transactions.
				let new_account = LocalAccount::generate(&mut rng);
				let user_account_creation_tx = root_account.sign_with_transaction_builder(
					tx_factory.create_user_account(new_account.public_key()),
				);
				let tx_hash = user_account_creation_tx.committed_hash();
				transaction_hashes.push(tx_hash);
				transactions.push(Transaction::UserTransaction(user_account_creation_tx));
			}

			// Group all transactions into an unsharded block for execution.
			let executable_transactions = ExecutableTransactions::Unsharded(
				transactions.into_iter().map(SignatureVerifiedTransaction::Valid).collect(),
			);
			let block = ExecutableBlock::new(block_id.clone(), executable_transactions);
			executor.execute_block_opt(block).await?;

			// Retrieve the executor's API interface and fetch the transaction by each hash.
			for hash in transaction_hashes {
				let _ = apis
					.transactions
					.get_transaction_by_hash_inner(&AcceptType::Bcs, hash.into())
					.await?;
			}
		}

		background_handle.abort();
		Ok(())
	}

	#[tokio::test]
	async fn test_set_finalized_block_height_get_fin_api() -> Result<(), anyhow::Error> {
		// Create an executor instance from the environment configuration.
		let config = Config::default();
		let (tx_sender, _tx_receiver) = mpsc::channel(16);
		let executor = Executor::try_from_config(config)?;
		let (context, background) = executor.background(tx_sender)?;
		let config = executor.config();
		let services = context.services();

		// Retrieve the executor's fin API instance
		let apis = services.get_fin_apis();

		let background_handle = tokio::spawn(background);

		// Initialize a root account using a predefined keypair and the test root address.
		let root_account = LocalAccount::new(
			aptos_test_root_address(),
			AccountKey::from_private_key(config.chain.maptos_private_key.clone()),
			0,
		);

		// Seed for random number generator, used here to generate predictable results in a test environment.
		let seed = [4u8; 32];
		let mut rng = ::rand::rngs::StdRng::from_seed(seed);

		// Create a transaction factory with the chain ID of the executor.
		let tx_factory = TransactionFactory::new(config.chain.maptos_chain_id.clone());
		let mut transaction_hashes = Vec::new();

		// Simulate the execution of multiple blocks.
		for _ in 0..3 {
			let block_id = HashValue::random(); // Generate a random block ID for each block.
			let block_metadata = executor
				.build_block_metadata(
					block_id.clone(),
					chrono::Utc::now().timestamp_micros() as u64,
				)
				.unwrap();

			// Generate new accounts and create a transaction for each block.
			let mut transactions = Vec::new();
			transactions.push(Transaction::BlockMetadata(block_metadata));
			let new_account = LocalAccount::generate(&mut rng);
			let user_account_creation_tx = root_account.sign_with_transaction_builder(
				tx_factory.create_user_account(new_account.public_key()),
			);
			let tx_hash = user_account_creation_tx.committed_hash();
			transaction_hashes.push(tx_hash);
			transactions.push(Transaction::UserTransaction(user_account_creation_tx));

			// Group all transactions into an unsharded block for execution.
			let executable_transactions = ExecutableTransactions::Unsharded(
				transactions.into_iter().map(SignatureVerifiedTransaction::Valid).collect(),
			);
			let block = ExecutableBlock::new(block_id.clone(), executable_transactions);
			executor.execute_block_opt(block).await?;
		}

		// Set the fin height
		executor.set_finalized_block_height(2)?;

		// Fetch the transaction in block 2
		let _ = apis
			.transactions
			.get_transaction_by_hash_inner(&AcceptType::Bcs, transaction_hashes[1].into())
			.await?;

		// The API method will not resolve because the transaction is "pending"
		// in the view of the finalized chain. Go through the context to check
		// that the transaction is not present in the fin state view.
		let context = apis.transactions.context.clone();
		let ledger_info = context.get_latest_ledger_info_wrapped()?;
		let opt =
			context.get_transaction_by_hash(transaction_hashes[2].into(), ledger_info.version())?;
		assert!(opt.is_none(), "transaction from opt block is found in the fin view");

		background_handle.abort();

		Ok(())
	}
}
