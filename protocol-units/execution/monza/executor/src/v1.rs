use crate::*;
use aptos_types::transaction::SignedTransaction;
use async_channel::Sender;
use maptos_opt_executor::Executor;
use movement_types::BlockCommitment;

#[derive(Clone)]
pub struct MonzaExecutorV1 {
	// this rwlock may be somewhat redundant
	pub executor: Executor,
	pub transaction_channel: Sender<SignedTransaction>,
}

impl MonzaExecutorV1 {
	pub fn new(executor: Executor, transaction_channel: Sender<SignedTransaction>) -> Self {
		Self { executor, transaction_channel }
	}

	pub async fn try_from_env(
		transaction_channel: Sender<SignedTransaction>,
	) -> Result<Self, anyhow::Error> {
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
		mode: FinalityMode,
		block: ExecutableBlock,
	) -> Result<BlockCommitment, anyhow::Error> {
		match mode {
			FinalityMode::Dyn => unimplemented!(),
			FinalityMode::Opt => {
				#[cfg(feature = "logging")]
				{
					tracing::debug!("Executing opt block: {:?}", block.block_id)
				}
				self.executor.execute_block(block).await
			},
			FinalityMode::Fin => unimplemented!(),
		}
	}

	/// Sets the transaction channel.
	fn set_tx_channel(
		&mut self,
		tx_channel: Sender<SignedTransaction>,
	) {
		self.transaction_channel = tx_channel;
	}

	/// Gets the API.
	fn get_api(&self, mode: FinalityMode) -> Apis {
		match mode {
			FinalityMode::Dyn => unimplemented!(),
			FinalityMode::Opt => self.executor.get_apis(),
			FinalityMode::Fin => unimplemented!(),
		}
	}

	/// Get block head height.
	async fn get_block_head_height(&self) -> Result<u64, anyhow::Error> {
		// ideally, this should read from the ledger
		Ok(1)
	}
}

#[cfg(test)]
mod tests {

	use std::collections::HashMap;

	use super::*;
	use aptos_api::{accept_type::AcceptType, transactions::SubmitTransactionPost};
	use aptos_crypto::{
		ed25519::{Ed25519PrivateKey, Ed25519Signature},
		HashValue, PrivateKey, Uniform,
	};
	use aptos_mempool::{MempoolClientRequest, MempoolClientSender};
	use aptos_sdk::{
		transaction_builder::TransactionFactory,
		types::{AccountKey, LocalAccount},
	};
	use aptos_storage_interface::state_view::DbStateViewAtVersion;
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
	use futures::channel::oneshot;
	use futures::SinkExt;
	use rand::SeedableRng;

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
		let executor = MonzaExecutorV1::try_from_env(tx).await?;
		let block_id = HashValue::random();
		let tx = SignatureVerifiedTransaction::Valid(Transaction::UserTransaction(
			create_signed_transaction(0),
		));
		let txs = ExecutableTransactions::Unsharded(vec![tx]);
		let block = ExecutableBlock::new(block_id.clone(), txs);
		executor.execute_block(FinalityMode::Opt, block).await?;
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

		let request = SubmitTransactionPost::Bcs(aptos_api::bcs_payload::Bcs(bcs_user_transaction));
		let api = executor.get_api(FinalityMode::Opt);
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

		let request = SubmitTransactionPost::Bcs(aptos_api::bcs_payload::Bcs(bcs_user_transaction));
		let api = executor.get_api(FinalityMode::Opt);
		api.transactions.submit_transaction(AcceptType::Bcs, request).await?;

		let received_transaction = rx.recv().await?;
		assert_eq!(received_transaction, comparison_user_transaction);

		// Now execute the block
		let block_id = HashValue::random();
		let tx =
			SignatureVerifiedTransaction::Valid(Transaction::UserTransaction(received_transaction));
		let txs = ExecutableTransactions::Unsharded(vec![tx]);
		let block = ExecutableBlock::new(block_id.clone(), txs);
		executor.execute_block(FinalityMode::Opt, block).await?;

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
			hash: HashValue,
			info: LedgerInfoWithSignatures,
			cur_ver: Version,
		}

		let (tx, rx) = async_channel::unbounded::<SignedTransaction>();
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
			let api = executor.get_api(FinalityMode::Opt);
			api.transactions.submit_transaction(AcceptType::Bcs, request).await?;

			let received_transaction = rx.recv().await?;
			assert_eq!(received_transaction, comparison_user_transaction);

			// Now execute the block
			let block_id = HashValue::random();
			let tx = SignatureVerifiedTransaction::Valid(Transaction::UserTransaction(
				received_transaction,
			));
			let txs = ExecutableTransactions::Unsharded(vec![tx]);
			let block = ExecutableBlock::new(block_id.clone(), txs);
			executor.execute_block(FinalityMode::Opt, block).await?;

			blockheight += 1;
			committed_blocks.insert(
				blockheight,
				Commit {
					hash: ledger_info_with_sigs.commit_info().executed_state_id(),
					info: ledger_info_with_sigs.clone(),
					cur_ver,
				},
			);
			commit_versions.push(cur_ver);
			cur_ver += txns_to_commit.len() as u64;
			blockheight += 1;
		}

		// Get the 3rd block back from the latest block
		let revert_block_num = blockheight - 3;
		let revert = committed_blocks.get(&revert_block_num).unwrap();

		// Get the version to revert to
		let version_to_revert = revert.cur_ver - 1;

		if let Some((max_blockheight, last_commit)) =
			committed_blocks.iter().max_by_key(|(&k, _)| k)
		{
			let db = executor.executor.db.clone();
			let mut db_writer = db.write_owned().await.writer.clone();
			db_writer.revert_commit(
				version_to_revert,
				last_commit.cur_ver,
				revert.hash,
				revert.info.clone(),
			)?;

			drop(db_writer);
		} else {
			panic!("No blocks to revert");
		}

		let db_reader = executor.executor.db.read_owned().await.reader.clone();
		let latest_version = db_reader.get_latest_version()?;
		assert_eq!(db_reader.get_latest_version().unwrap(), version_to_revert - 1);

		services_handle.abort();
		background_handle.abort();
		Ok(())
	}

	#[tokio::test]
	async fn test_execute_block_state_get_api() -> Result<(), anyhow::Error> {
		// Create an executor instance from the environment configuration.
		let executor = Executor::try_from_env()?;

		// Initialize a root account using a predefined keypair and the test root address.
		let root_account = LocalAccount::new(
			aptos_test_root_address(),
			AccountKey::from_private_key(executor.aptos_config.aptos_private_key.clone()),
			0,
		);

		// Seed for random number generator, used here to generate predictable results in a test environment.
		let seed = [3u8; 32];
		let mut rng = ::rand::rngs::StdRng::from_seed(seed);

		// Create a transaction factory with the chain ID of the executor.
		let tx_factory = TransactionFactory::new(executor.aptos_config.chain_id.clone());

		// Simulate the execution of multiple blocks.
		for _ in 0..10 {
			// For example, create and execute 3 blocks.
			let block_id = HashValue::random(); // Generate a random block ID for each block.

			// Generate new accounts and create transactions for each block.
			let mut transactions = Vec::new();
			let mut transaction_hashes = Vec::new();
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
			executor.execute_block(block).await?;

			// Retrieve the executor's API interface and fetch the transaction by each hash.
			let apis = executor.get_apis();
			for hash in transaction_hashes {
				let _ = apis
					.transactions
					.get_transaction_by_hash_inner(&AcceptType::Bcs, hash.into())
					.await?;
			}
		}

		Ok(())
	}
}
