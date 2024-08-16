use super::Executor;
use aptos_api::Context;
use aptos_crypto::HashValue;
use aptos_executor_types::BlockExecutorTrait;
use aptos_types::transaction::signature_verified_transaction::into_signature_verified_block;
use aptos_types::{
	account_address::AccountAddress,
	aggregate_signature::AggregateSignature,
	block_executor::{
		config::BlockExecutorConfigFromOnchain,
		partitioner::{ExecutableBlock, ExecutableTransactions},
	},
	block_info::BlockInfo,
	block_metadata::BlockMetadata,
	epoch_state::EpochState,
	ledger_info::{LedgerInfo, LedgerInfoWithSignatures},
	transaction::{Transaction, Version},
	validator_verifier::{ValidatorConsensusInfo, ValidatorVerifier},
};
use movement_types::{BlockCommitment, Commitment, Id};
use std::sync::Arc;
use tracing::{debug, debug_span, info};

impl Executor {
	pub async fn execute_block(
		&self,
		block: ExecutableBlock,
	) -> Result<BlockCommitment, anyhow::Error> {
		let (block_metadata, block, senders_and_sequence_numbers) = {
			// get the block metadata transaction
			let metadata_access_block = block.transactions.clone();
			let metadata_access_transactions = metadata_access_block.into_txns();
			let first_signed = metadata_access_transactions
				.first()
				.ok_or(anyhow::anyhow!("Block must contain a block metadata transaction"))?;
			// cloning is cheaper than moving the array
			let block_metadata = match first_signed.clone().into_inner() {
				Transaction::BlockMetadata(metadata) => metadata.clone(),
				_ => {
					anyhow::bail!("First transaction in block must be a block metadata transaction")
				}
			};

			// senders and sequence numbers
			let senders_and_sequence_numbers = metadata_access_transactions
				.iter()
				.map(|transaction| match transaction.clone().into_inner() {
					Transaction::UserTransaction(transaction) => {
						(transaction.sender(), transaction.sequence_number())
					}
					_ => (AccountAddress::ZERO, 0),
				})
				.collect::<Vec<(AccountAddress, u64)>>();

			// reconstruct the block
			let block = ExecutableBlock::new(
				block.block_id.clone(),
				ExecutableTransactions::Unsharded(metadata_access_transactions),
			);

			(block_metadata, block, senders_and_sequence_numbers)
		};

		let block_id = block.block_id.clone();
		let parent_block_id = self.block_executor.committed_block_id();

		let block_executor_clone = self.block_executor.clone();
		let state_compute = tokio::task::spawn_blocking(move || {
			block_executor_clone.execute_block(
				block,
				parent_block_id,
				BlockExecutorConfigFromOnchain::new_no_block_limit(),
			)
		})
		.await??;

		debug!("Block execution compute the following state: {:?}", state_compute);

		let version = state_compute.version();
		debug!("Block execution computed the following version: {:?}", version);
		let (epoch, round) = (block_metadata.epoch(), block_metadata.round());

		let ledger_info_with_sigs = self.ledger_info_with_sigs(
			epoch,
			round,
			block_id.clone(),
			block_metadata.timestamp_usecs(),
			state_compute.root_hash(),
			version,
		);
		let block_executor_clone = self.block_executor.clone();
		tokio::task::spawn_blocking(move || {
			block_executor_clone.commit_blocks(vec![block_id], ledger_info_with_sigs)
		})
		.await??;

		let proof = {
			let reader = self.db.reader.clone();
			reader.get_state_proof(version)?
		};

		// Context has a reach-around to the db so the block height should
		// have been updated to the most recently committed block.
		// Race conditions, anyone?
		let block_height = self.get_block_head_height()?;

		let commitment = Commitment::digest_state_proof(&proof);
		Ok(BlockCommitment {
			block_id: Id(*block_id.clone()),
			commitment,
			height: block_height.into(),
		})
	}

	pub fn get_block_head_height(&self) -> Result<u64, anyhow::Error> {
		let ledger_info = self.context.get_latest_ledger_info_wrapped()?;
		Ok(ledger_info.block_height.into())
	}

	pub fn revert_block_head_to(&self, block_height: u64) -> Result<(), anyhow::Error> {
		let (_start_ver, end_ver, block_event) =
			self.db.reader.get_block_info_by_height(block_height)?;
		let block_info = BlockInfo::new(
			block_event.epoch(),
			block_event.round(),
			block_event.hash()?,
			self.db.reader.get_accumulator_root_hash(end_ver)?,
			end_ver,
			block_event.proposed_time(),
			None,
		);
		let ledger_info = LedgerInfo::new(block_info, HashValue::zero());
		let aggregate_signature = AggregateSignature::empty();
		let ledger_info = LedgerInfoWithSignatures::new(ledger_info, aggregate_signature);
		self.db.writer.revert_commit(&ledger_info)?;
		// Reset the executor state to the reverted storage
		self.block_executor.reset()?;
		Ok(())
	}

	pub fn context(&self) -> Arc<Context> {
		self.context.clone()
	}

	/// Gets the next epoch and round.
	pub async fn get_next_epoch_and_round(&self) -> Result<(u64, u64), anyhow::Error> {
		let epoch = self.db.reader.get_latest_ledger_info()?.ledger_info().next_block_epoch();
		let round = self.db.reader.get_latest_ledger_info()?.ledger_info().round();
		Ok((epoch, round))
	}

	/// Gets the timestamp of the last state.
	pub async fn get_last_state_timestamp_micros(&self) -> Result<u64, anyhow::Error> {
		let ledger_info = self.db.reader.get_latest_ledger_info()?;
		Ok(ledger_info.ledger_info().timestamp_usecs())
	}

	pub async fn rollover_genesis(&self, timestamp: u64) -> Result<(), anyhow::Error> {
		let (epoch, round) = self.get_next_epoch_and_round().await?;
		let block_id = HashValue::random();

		// genesis timestamp should always be 0
		let genesis_timestamp = self.get_last_state_timestamp_micros().await?;
		info!(
			"Rollover genesis: epoch: {}, round: {}, block_id: {}, genesis timestamp {}",
			epoch, round, block_id, genesis_timestamp
		);

		let block_metadata = Transaction::BlockMetadata(BlockMetadata::new(
			block_id,
			epoch,
			round,
			self.signer.author(),
			vec![],
			vec![],
			timestamp,
		));
		let txs =
			ExecutableTransactions::Unsharded(into_signature_verified_block(vec![block_metadata]));
		let block = ExecutableBlock::new(block_id.clone(), txs);
		self.execute_block(block).await?;
		Ok(())
	}

	/// Rollover the genesis block.
	/// This should only be used for testing. The data availability layer should provide an initial transaction that rolls over the genesis block.
	pub async fn rollover_genesis_now(&self) -> Result<(), anyhow::Error> {
		// rollover timestamp needs to be within the epoch, by  default above this is one hour, so below is 59 minutes
		let rollover_timestamp = chrono::Utc::now().timestamp_micros() as u64;
		self.rollover_genesis(
			rollover_timestamp,
			// rollover_timestamp - (59 * 60 * 1000 * 1000), // 60 minutes
		)
		.await?;
		Ok(())
	}

	pub fn ledger_info_with_sigs(
		&self,
		epoch: u64,
		round: u64,
		block_id: HashValue,
		timestamp_microseconds: u64,
		root_hash: HashValue,
		version: Version,
	) -> LedgerInfoWithSignatures {
		let block_info = BlockInfo::new(
			epoch,
			round,
			block_id,
			root_hash,
			version,
			timestamp_microseconds,
			Some(EpochState {
				epoch,
				verifier: ValidatorVerifier::new(vec![ValidatorConsensusInfo::new(
					self.signer.author(),
					self.signer.public_key(),
					100_000_000,
				)]),
			}),
		);
		let ledger_info = LedgerInfo::new(
			block_info,
			HashValue::zero(), /* consensus_data_hash, doesn't matter */
		);
		LedgerInfoWithSignatures::new(
			ledger_info,
			AggregateSignature::empty(), /* signatures */
		)
	}
}

#[cfg(test)]
mod tests {

	use super::*;
	use aptos_api::accept_type::AcceptType;
	use aptos_crypto::{
		ed25519::{Ed25519PrivateKey, Ed25519Signature},
		HashValue, PrivateKey, Uniform,
	};
	use aptos_sdk::{
		transaction_builder::TransactionFactory,
		types::{AccountKey, LocalAccount},
	};
	use aptos_storage_interface::state_view::DbStateViewAtVersion;
	use aptos_types::{
		account_address::AccountAddress,
		account_config::{aptos_test_root_address, AccountResource},
		block_executor::partitioner::ExecutableTransactions,
		block_metadata::BlockMetadata,
		chain_id::ChainId,
		state_store::MoveResourceExt,
		transaction::signature_verified_transaction::{
			into_signature_verified_block, SignatureVerifiedTransaction,
		},
		transaction::{RawTransaction, Script, SignedTransaction, Transaction, TransactionPayload},
	};
	use rand::SeedableRng;

	fn create_signed_transaction(gas_unit_price: u64, chain_id: ChainId) -> SignedTransaction {
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
			chain_id, // This is the value used in aptos testing code.
		);
		SignedTransaction::new(raw_transaction, public_key, Ed25519Signature::dummy_signature())
	}

	#[tokio::test]
	async fn test_execute_block() -> Result<(), anyhow::Error> {
		let private_key = Ed25519PrivateKey::generate_for_testing();
		let (executor, _tempdir) = Executor::try_test_default(private_key.clone())?;
		let block_id = HashValue::random();
		let block_metadata = Transaction::BlockMetadata(BlockMetadata::new(
			block_id,
			0,
			0,
			executor.signer.author(),
			vec![],
			vec![],
			chrono::Utc::now().timestamp_micros() as u64,
		));
		let tx = SignatureVerifiedTransaction::Valid(Transaction::UserTransaction(
			create_signed_transaction(0, executor.maptos_config.chain.maptos_chain_id.clone()),
		));
		let txs = ExecutableTransactions::Unsharded(vec![
			SignatureVerifiedTransaction::Valid(block_metadata),
			tx,
		]);
		let block = ExecutableBlock::new(block_id.clone(), txs);
		executor.execute_block(block).await?;
		Ok(())
	}

	// https://github.com/movementlabsxyz/aptos-core/blob/ea91067b81f9673547417bff9c70d5a2fe1b0e7b/execution/executor-test-helpers/src/integration_test_impl.rs#L535
	#[tracing_test::traced_test]
	#[tokio::test]
	async fn test_execute_block_state_db() -> Result<(), anyhow::Error> {
		// use aptos_logger::{Level, Logger};
		// Logger::builder().level(Level::Info).build();

		// Create an executor instance from the environment configuration.
		let private_key = Ed25519PrivateKey::generate_for_testing();
		let (executor, _tempdir) = Executor::try_test_default(private_key.clone())?;
		executor.rollover_genesis_now().await?;

		// Initialize a root account using a predefined keypair and the test root address.
		let root_account = LocalAccount::new(
			aptos_test_root_address(),
			AccountKey::from_private_key(executor.maptos_config.chain.maptos_private_key.clone()),
			0,
		);

		// Seed for random number generator, used here to generate predictable results in a test environment.
		let seed = [3u8; 32];
		let mut rng = ::rand::rngs::StdRng::from_seed(seed);

		// Loop to simulate the execution of multiple blocks.
		for i in 0..10 {
			let (epoch, round) = executor.get_next_epoch_and_round().await?;

			// Generate a random block ID.
			let block_id = HashValue::random();
			// Clone the signer from the executor for signing the metadata.
			let signer = executor.signer.clone();
			// Get the current time in microseconds for the block timestamp.
			let current_time_microseconds = chrono::Utc::now().timestamp_micros() as u64;

			// Create a transaction factory with the chain ID of the executor, used for creating transactions.
			let tx_factory =
				TransactionFactory::new(executor.maptos_config.chain.maptos_chain_id.clone())
					.with_transaction_expiration_time(
						current_time_microseconds, // current_time_microseconds + (i * 1000 * 1000 * 60 * 30) + 30,
					);

			// Create a block metadata transaction.
			let block_metadata = Transaction::BlockMetadata(BlockMetadata::new(
				block_id,
				epoch,
				round,
				signer.author(),
				vec![],
				vec![],
				current_time_microseconds,
				// ! below doesn't work, i.e., we can't roll over epochs
				// current_time_microseconds + (i * 1000 * 1000 * 60 * 30), // 30 minutes later, thus every other will be across an epoch
			));

			// Generate a new account for transaction tests.
			let new_account = LocalAccount::generate(&mut rng);
			let new_account_address = new_account.address();

			// Create a user account creation transaction.
			let user_account_creation_tx = root_account.sign_with_transaction_builder(
				tx_factory.create_user_account(new_account.public_key()),
			);

			// Create a mint transaction to provide the new account with some initial balance.
			let mint_tx = root_account
				.sign_with_transaction_builder(tx_factory.mint(new_account.address(), 2000));
			// Store the hash of the committed transaction for later verification.
			let mint_tx_hash = mint_tx.committed_hash();

			// Block Metadata
			let transactions =
				ExecutableTransactions::Unsharded(into_signature_verified_block(vec![
					block_metadata,
					Transaction::UserTransaction(user_account_creation_tx),
					Transaction::UserTransaction(mint_tx),
				]));
			debug!("Number of transactions: {}", transactions.num_transactions());
			let block = ExecutableBlock::new(block_id.clone(), transactions);
			let block_commitment = executor.execute_block(block).await?;

			// Access the database reader to verify state after execution.
			let db_reader = executor.db.reader.clone();
			// Get the latest version of the blockchain state from the database.
			let latest_version = db_reader.get_synced_version()?;
			// Verify the transaction by its hash to ensure it was committed.
			let transaction_result =
				db_reader.get_transaction_by_hash(mint_tx_hash, latest_version, false)?;
			assert!(transaction_result.is_some());

			// Create a state view at the latest version to inspect account states.
			let state_view = db_reader.state_view_at_version(Some(latest_version))?;
			// Access the state view of the new account to verify its state and existence.
			let _account_resource =
				AccountResource::fetch_move_resource(&state_view, &new_account_address)?.unwrap();

			// Check the commitment against state proof
			let state_proof = db_reader.get_state_proof(latest_version)?;
			let expected_commitment = Commitment::digest_state_proof(&state_proof);
			assert_eq!(block_commitment.height, i + 2);
			assert_eq!(block_commitment.commitment, expected_commitment);
		}

		Ok(())
	}

	#[tokio::test]
	async fn test_execute_block_state_get_api() -> Result<(), anyhow::Error> {
		// Create an executor instance from the environment configuration.
		let private_key = Ed25519PrivateKey::generate_for_testing();
		let (executor, _tempdir) = Executor::try_test_default(private_key.clone())?;
		executor.rollover_genesis_now().await?;

		// Initialize a root account using a predefined keypair and the test root address.
		let root_account = LocalAccount::new(
			aptos_test_root_address(),
			AccountKey::from_private_key(executor.maptos_config.chain.maptos_private_key.clone()),
			0,
		);

		// Seed for random number generator, used here to generate predictable results in a test environment.
		let seed = [3u8; 32];
		let mut rng = ::rand::rngs::StdRng::from_seed(seed);

		// Create a transaction factory with the chain ID of the executor.
		let tx_factory =
			TransactionFactory::new(executor.maptos_config.chain.maptos_chain_id.clone());

		// Simulate the execution of multiple blocks.
		for _ in 0..10 {
			// For example, create and execute 3 blocks.
			let (epoch, round) = executor.get_next_epoch_and_round().await?;

			let block_id = HashValue::random(); // Generate a random block ID for each block.

			// Clone the signer from the executor for signing the metadata.
			let signer = executor.signer.clone();
			// Get the current time in microseconds for the block timestamp.
			let current_time_microseconds = chrono::Utc::now().timestamp_micros() as u64;

			// Create a block metadata transaction.
			let block_metadata = Transaction::BlockMetadata(BlockMetadata::new(
				block_id,
				epoch,
				round,
				signer.author(),
				vec![],
				vec![],
				current_time_microseconds,
			));

			// Generate new accounts and create transactions for each block.
			let mut transactions = Vec::new();
			let mut transaction_hashes = Vec::new();
			transactions.push(block_metadata.clone());
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
