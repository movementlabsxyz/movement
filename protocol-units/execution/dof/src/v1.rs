use crate::{BlockMetadata, DynOptFinExecutor, ExecutableBlock, HashValue, SignedTransaction};
use aptos_api::runtime::Apis;
use maptos_fin_view::FinalityView;
use maptos_opt_executor::Executor as OptExecutor;
use movement_types::BlockCommitment;

use async_channel::Sender;
use async_trait::async_trait;
use tracing::debug;

#[derive(Clone)]
pub struct Executor {
	pub executor: OptExecutor,
	finality_view: FinalityView,
	pub transaction_channel: Sender<SignedTransaction>,
}

impl Executor {
	pub fn new(
		executor: OptExecutor,
		finality_view: FinalityView,
		transaction_channel: Sender<SignedTransaction>,
	) -> Self {
		Self { executor, finality_view, transaction_channel }
	}

	pub fn try_from_config(
		transaction_channel: Sender<SignedTransaction>,
		config: maptos_execution_util::config::Config,
	) -> Result<Self, anyhow::Error> {
		let executor = OptExecutor::try_from_config(&config.clone())?;
		let finality_view = FinalityView::try_from_config(
			executor.db.reader.clone(),
			executor.mempool_client_sender.clone(),
			config,
		)?;
		Ok(Self::new(executor, finality_view, transaction_channel))
	}
}

#[async_trait]
impl DynOptFinExecutor for Executor {
	/// Runs the service.
	async fn run_service(&self) -> Result<(), anyhow::Error> {
		tokio::try_join!(self.executor.run_service(), self.finality_view.run_service(),)?;
		Ok(())
	}

	/// Runs the necessary background tasks.
	async fn run_background_tasks(&self) -> Result<(), anyhow::Error> {
		loop {
			// readers should be able to run concurrently
			self.executor.tick_transaction_pipe(self.transaction_channel.clone()).await?;
		}
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

	/// Sets the transaction channel.
	fn set_tx_channel(&mut self, tx_channel: Sender<SignedTransaction>) {
		self.transaction_channel = tx_channel;
	}

	fn get_opt_apis(&self) -> Apis {
		self.executor.get_apis()
	}

	fn get_fin_apis(&self) -> Apis {
		self.finality_view.get_apis()
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
		let signer = &self.executor.signer;

		// Create a block metadata transaction.
		Ok(BlockMetadata::new(block_id, epoch, round, signer.author(), vec![], vec![], timestamp))
	}

	/// Rollover the genesis block
	async fn rollover_genesis_block(&self) -> Result<(), anyhow::Error> {
		self.executor.rollover_genesis_now().await
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
		ledger_info::LedgerInfoWithSignatures,
		transaction::{
			signature_verified_transaction::SignatureVerifiedTransaction, RawTransaction, Script,
			SignedTransaction, Transaction, TransactionPayload, Version,
		},
	};
	use maptos_execution_util::config::Config;

	use rand::SeedableRng;

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
		let (tx, _rx) = async_channel::unbounded();
		let executor = Executor::try_from_config(tx, config)?;
		let block_id = HashValue::random();
		let block_metadata = executor
			.build_block_metadata(block_id.clone(), chrono::Utc::now().timestamp_micros() as u64)
			.await
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
		let (tx, rx) = async_channel::unbounded();
		let executor = Executor::try_from_config(tx, config)?;
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
		let api = executor.get_opt_apis();
		api.transactions.submit_transaction(AcceptType::Bcs, request).await?;

		services_handle.abort();
		background_handle.abort();
		let received_transaction = rx.recv().await?;
		assert_eq!(received_transaction, comparison_user_transaction);

		Ok(())
	}

	#[tokio::test]
	async fn test_pipe_transactions_from_api_and_execute() -> Result<(), anyhow::Error> {
		let config = Config::default();
		let (tx, rx) = async_channel::unbounded();
		let executor = Executor::try_from_config(tx, config)?;
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
		let api = executor.get_opt_apis();
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

	#[tokio::test]
	async fn test_revert_chain_state_at_nth_commit() -> Result<(), anyhow::Error> {
		use aptos_db::db::test_helper::arb_blocks_to_commit_with_block_nums;
		use aptos_proptest_helpers::ValueGenerator;

		#[derive(Debug)]
		struct Commit {
			info: LedgerInfoWithSignatures,
			cur_ver: Version,
		}

		let config = Config::default();
		let (tx, rx) = async_channel::unbounded::<SignedTransaction>();
		let executor = Executor::try_from_config(tx, config)?;
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
		let mut committed_blocks = HashMap::new();

		let mut val_generator = ValueGenerator::new();
		// set range of min and max blocks to 5 to always gen 5 blocks
		let (blocks, _) = val_generator.generate(arb_blocks_to_commit_with_block_nums(5, 5));
		let mut blockheight = 0;
		let mut cur_ver: Version = 0;
		let mut commit_versions = vec![];

		for (txns_to_commit, ledger_info_with_sigs) in &blocks {
			let user_transaction = create_signed_transaction(0);
			let comparison_user_transaction = user_transaction.clone();
			let bcs_user_transaction = bcs::to_bytes(&user_transaction)?;

			let request =
				SubmitTransactionPost::Bcs(aptos_api::bcs_payload::Bcs(bcs_user_transaction));
			let api = executor.get_opt_apis();
			api.transactions.submit_transaction(AcceptType::Bcs, request).await?;

			let received_transaction = rx.recv().await?;
			assert_eq!(received_transaction, comparison_user_transaction);

			// Now execute the block
			let block_id = HashValue::random();
			let block_metadata = executor
				.build_block_metadata(
					block_id.clone(),
					chrono::Utc::now().timestamp_micros() as u64,
				)
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
			executor.execute_block_opt(block).await?;

			blockheight += 1;
			cur_ver += txns_to_commit.len() as u64;
			committed_blocks.insert(
				blockheight,
				Commit {
					info: ledger_info_with_sigs.clone(),
					cur_ver,
				},
			);
			commit_versions.push(cur_ver);
			//blockheight += 1;
		}

		// Get the 3rd block back from the latest block
		let revert_block_num = blockheight - 3;
		let revert = committed_blocks.get(&revert_block_num).unwrap();

		// Get the version to revert to
		let version_to_revert_to = revert.cur_ver;

		if let Some((_max_blockheight, last_commit)) =
			committed_blocks.iter().max_by_key(|(&k, _)| k)
		{
			let db_writer = executor.executor.db.writer.clone();
			db_writer.revert_commit(&revert.info)?;
		} else {
			panic!("No blocks to revert");
		}

		let db_reader = executor.executor.db.reader.clone();
		let latest_version = db_reader.get_latest_version()?;
		assert_eq!(latest_version, version_to_revert_to);

		services_handle.abort();
		background_handle.abort();
		Ok(())
	}

	#[tokio::test]
	async fn test_execute_block_state_get_api() -> Result<(), anyhow::Error> {
		// Create an executor instance from the environment configuration.
		let (tx, _rx) = async_channel::unbounded::<SignedTransaction>();
		let config = Config::default();
		let chain_config = config.chain.clone();
		let executor = Executor::try_from_config(tx, config)?;

		// Initialize a root account using a predefined keypair and the test root address.
		let root_account = LocalAccount::new(
			aptos_test_root_address(),
			AccountKey::from_private_key(chain_config.maptos_private_key),
			0,
		);

		// Seed for random number generator, used here to generate predictable results in a test environment.
		let seed = [3u8; 32];
		let mut rng = ::rand::rngs::StdRng::from_seed(seed);

		// Create a transaction factory with the chain ID of the executor.
		let tx_factory = TransactionFactory::new(chain_config.maptos_chain_id);

		// Simulate the execution of multiple blocks.
		for _ in 0..10 {
			// For example, create and execute 3 blocks.
			let block_id = HashValue::random(); // Generate a random block ID for each block.
			let block_metadata = executor
				.build_block_metadata(
					block_id.clone(),
					chrono::Utc::now().timestamp_micros() as u64,
				)
				.await
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
				let tx_hash = user_account_creation_tx.clone().committed_hash();
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
			let apis = executor.get_opt_apis();
			for hash in transaction_hashes {
				let _ = apis
					.transactions
					.get_transaction_by_hash_inner(&AcceptType::Bcs, hash.into())
					.await?;
			}
		}

		Ok(())
	}

	#[tokio::test]
	async fn test_set_finalized_block_height_get_fin_api() -> Result<(), anyhow::Error> {
		// Create an executor instance from the environment configuration.
		let (tx, _rx) = async_channel::unbounded::<SignedTransaction>();
		let config = Config::default();
		let chain_config = config.chain.clone();
		let executor = Executor::try_from_config(tx, config)?;

		// Initialize a root account using a predefined keypair and the test root address.
		let root_account = LocalAccount::new(
			aptos_test_root_address(),
			AccountKey::from_private_key(chain_config.maptos_private_key.clone()),
			0,
		);

		// Seed for random number generator, used here to generate predictable results in a test environment.
		let seed = [4u8; 32];
		let mut rng = ::rand::rngs::StdRng::from_seed(seed);

		// Create a transaction factory with the chain ID of the executor.
		let tx_factory = TransactionFactory::new(chain_config.maptos_chain_id);
		let mut transaction_hashes = Vec::new();

		// Simulate the execution of multiple blocks.
		for _ in 0..3 {
			let block_id = HashValue::random(); // Generate a random block ID for each block.
			let block_metadata = executor
				.build_block_metadata(
					block_id.clone(),
					chrono::Utc::now().timestamp_micros() as u64,
				)
				.await
				.unwrap();

			// Generate new accounts and create a transaction for each block.
			let mut transactions = Vec::new();
			transactions.push(Transaction::BlockMetadata(block_metadata));
			let new_account = LocalAccount::generate(&mut rng);
			let user_account_creation_tx = root_account.sign_with_transaction_builder(
				tx_factory.create_user_account(new_account.public_key()),
			);
			let tx_hash = user_account_creation_tx.clone().committed_hash();
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

		// Retrieve the executor's fin API instance
		let apis = executor.get_fin_apis();

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

		Ok(())
	}
}
