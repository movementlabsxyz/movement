use super::Executor;
use crate::executor::ExecutionState;
use crate::executor::TxExecutionResult;
use aptos_crypto::HashValue;
use aptos_executor_types::{BlockExecutorTrait, StateComputeResult};
use aptos_sdk::types::account_address::AccountAddress;
use aptos_sdk::types::AccountKey;
use aptos_types::account_config::aptos_test_root_address;
use aptos_types::{
	aggregate_signature::AggregateSignature,
	block_executor::{
		config::BlockExecutorConfigFromOnchain,
		partitioner::{ExecutableBlock, ExecutableTransactions},
	},
	block_info::BlockInfo,
	epoch_state::EpochState,
	ledger_info::{LedgerInfo, LedgerInfoWithSignatures},
	transaction::{Transaction, Version},
	validator_verifier::{ValidatorConsensusInfo, ValidatorVerifier},
};
use movement_types::block::{BlockCommitment, Commitment, Id};
use tracing::debug;
use tracing::info;

impl Executor {
	pub fn execute_block(
		&mut self,
		block: ExecutableBlock,
	) -> Result<(BlockCommitment, ExecutionState), anyhow::Error> {
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
					Transaction::UserTransaction(transaction) => (
						transaction.committed_hash(),
						transaction.sender(),
						transaction.sequence_number(),
					),
					_ => (HashValue::zero(), AccountAddress::ZERO, 0u64),
				})
				.collect::<Vec<(HashValue, AccountAddress, u64)>>();

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
		let state_compute: StateComputeResult = block_executor_clone.execute_block(
			block,
			parent_block_id,
			BlockExecutorConfigFromOnchain::new_no_block_limit(),
		)?;

		let tx_execution_results =
			TxExecutionResult::merge_result(senders_and_sequence_numbers, &state_compute);

		debug!("Block tx execution: {:?}", tx_execution_results);
		info!("Block execution compute the following state: {:?}", state_compute);

		let version = state_compute.version();
		info!("Block execution computed the following version: {:?}", version);
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
		block_executor_clone.commit_blocks(vec![block_id], ledger_info_with_sigs.clone())?;

		// Context has a reach-around to the db so the block height should
		// have been updated to the most recently committed block.
		// Race conditions, anyone?
		let block_height = self.get_block_head_height()?;

		let new_execution_state = ExecutionState::build(&ledger_info_with_sigs, block_height);

		// commit mempool transactions
		self.mempool_tx_exec_result_sender.send(tx_execution_results)?;

		let proof = self.db().reader.get_state_proof(version)?;

		debug!("proof :{proof:?}");

		let commitment = BlockCommitment::new(
			block_height.into(),
			Id::new(*block_id.clone()),
			Commitment::digest_state_proof(&proof),
		);
		Ok((commitment, new_execution_state))
	}

	pub fn get_block_head_height(&self) -> Result<u64, anyhow::Error> {
		let ledger_info = self.db().reader.get_latest_ledger_info()?;
		info!("get_block_head_height lastest ledger_info :{ledger_info:?}");
		let (_, _, new_block_event) = self
			.db()
			.reader
			.get_block_info_by_version(ledger_info.ledger_info().version())?;
		Ok(new_block_event.height)
	}

	pub fn get_commitment_for_height(&self, height: u64) -> Result<BlockCommitment, anyhow::Error> {
		let (_block_start_version, block_end_version, _block_event) =
			self.db().reader.get_block_info_by_height(height)?;
		let proof = self.db().reader.get_state_proof(block_end_version)?;

		let block_id = proof.latest_ledger_info().consensus_block_id();

		let commitment = Commitment::digest_state_proof(&proof);
		Ok(BlockCommitment::new(height.into(), Id::new(*block_id.clone()), commitment))
	}

	pub fn get_commitment_for_version(
		&self,
		version: u64,
	) -> Result<BlockCommitment, anyhow::Error> {
		let (_block_start_version, block_end_version, block_event) =
			self.db().reader.get_block_info_by_version(version)?;
		let height = block_event.height;
		let proof = self.db().reader.get_state_proof(block_end_version)?;

		let block_id = proof.latest_ledger_info().consensus_block_id();

		let commitment = Commitment::digest_state_proof(&proof);
		Ok(BlockCommitment::new(height.into(), Id::new(*block_id.clone()), commitment))
	}

	pub async fn revert_block_head_to(&self, block_height: u64) -> Result<(), anyhow::Error> {
		let (_start_ver, end_ver, block_event) =
			self.db().reader.get_block_info_by_height(block_height)?;
		let block_info = BlockInfo::new(
			block_event.epoch(),
			block_event.round(),
			block_event.hash()?,
			self.db().reader.get_accumulator_root_hash(end_ver)?,
			end_ver,
			block_event.proposed_time(),
			None,
		);
		let ledger_info = LedgerInfo::new(block_info, HashValue::zero());
		let aggregate_signature = AggregateSignature::empty();
		let ledger_info = LedgerInfoWithSignatures::new(ledger_info, aggregate_signature);
		let db_writer = self.db().writer.clone();
		let ledger_info_copy = ledger_info.clone();
		tokio::task::spawn_blocking(move || db_writer.revert_commit(&ledger_info_copy)).await??;
		// Reset the executor state to the reverted storage
		self.block_executor.reset()?;
		Ok(())
	}

	/// Gets the next epoch and round.
	pub fn get_next_epoch_and_round(&self) -> Result<(u64, u64), anyhow::Error> {
		let epoch = self.db().reader.get_latest_ledger_info()?.ledger_info().next_block_epoch();
		let round = self.db().reader.get_latest_ledger_info()?.ledger_info().round();
		Ok((epoch, round))
	}

	/// Gets the timestamp of the last state.
	pub fn get_last_state_timestamp_micros(&self) -> Result<u64, anyhow::Error> {
		let ledger_info = self.db().reader.get_latest_ledger_info()?;
		Ok(ledger_info.ledger_info().timestamp_usecs())
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
		let next_epoch = if version >= self.config().chain.dont_increase_epoch_until_version {
			epoch + 1
		} else {
			epoch
		};

		let block_info = BlockInfo::new(
			epoch,
			round,
			block_id,
			root_hash,
			version,
			timestamp_microseconds,
			Some(EpochState {
				epoch: next_epoch, // we now increase the epoch
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
	use crate::executor::TxExecutionResult;
	use crate::Service;
	use aptos_api::accept_type::AcceptType;
	use aptos_crypto::{
		ed25519::{Ed25519PrivateKey, Ed25519Signature},
		HashValue, PrivateKey, Uniform,
	};
	use aptos_mempool::MempoolClientRequest;
	use aptos_sdk::types::account_config::aptos_test_root_address;
	use aptos_sdk::{
		transaction_builder::TransactionFactory,
		types::{AccountKey, LocalAccount},
	};

	use aptos_storage_interface::state_view::DbStateViewAtVersion;
	use aptos_types::{
		account_address::AccountAddress,
		account_config::AccountResource,
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
	use tokio::sync::mpsc::unbounded_channel;
	use tracing::debug;

	fn create_signed_transaction(sequence_number: u64, chain_id: ChainId) -> SignedTransaction {
		let private_key = Ed25519PrivateKey::generate_for_testing();
		let public_key = private_key.public_key();
		let transaction_payload = TransactionPayload::Script(Script::new(vec![0], vec![], vec![]));
		let raw_transaction = RawTransaction::new(
			AccountAddress::random(),
			sequence_number,
			transaction_payload,
			0,
			0,
			0,
			chain_id, // This is the value used in aptos testing code.
		);
		SignedTransaction::new(raw_transaction, public_key, Ed25519Signature::dummy_signature())
	}

	#[tokio::test]
	async fn test_execute_block() -> Result<(), anyhow::Error> {
		let private_key = Ed25519PrivateKey::generate_for_testing();
		let (tx_sender, _tx_receiver) = futures::channel::mpsc::channel::<MempoolClientRequest>(10);
		let (mempool_tx_exec_result_sender, mempool_commit_tx_receiver) =
			unbounded_channel::<Vec<TxExecutionResult>>();

		let (mut executor, _tempdir) =
			Executor::try_test_default(private_key, mempool_tx_exec_result_sender).await?;
		let (context, _transaction_pipe) =
			executor.background(mempool_commit_tx_receiver, tx_sender)?;
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
			create_signed_transaction(0, context.config().chain.maptos_chain_id.clone()),
		));
		let txs = ExecutableTransactions::Unsharded(vec![
			SignatureVerifiedTransaction::Valid(block_metadata),
			tx,
		]);
		let block = ExecutableBlock::new(block_id.clone(), txs);
		executor.execute_block(block)?;
		Ok(())
	}

	// https://github.com/movementlabsxyz/aptos-core/blob/ea91067b81f9673547417bff9c70d5a2fe1b0e7b/execution/executor-test-helpers/src/integration_test_impl.rs#L535
	//	#[tracing_test::traced_test]
	#[tokio::test]
	//	#[ignore]
	async fn test_execute_block_state_db() -> Result<(), anyhow::Error> {
		// use aptos_logger::{Level, Logger};
		// Logger::builder().level(Level::Debug).build();

		// Create an executor instance from the environment configuration.
		let private_key = Ed25519PrivateKey::generate_for_testing();

		let (tx_sender, _tx_receiver) = futures::channel::mpsc::channel::<MempoolClientRequest>(10);
		let (mempool_tx_exec_result_sender, mempool_commit_tx_receiver) =
			unbounded_channel::<Vec<TxExecutionResult>>();

		let (mut executor, _tempdir) =
			Executor::try_test_default(private_key, mempool_tx_exec_result_sender).await?;
		let (context, _transaction_pipe) =
			executor.background(mempool_commit_tx_receiver, tx_sender)?;

		// Initialize a root account using a predefined keypair and the test root address.
		// get the raw private key
		let raw_private_key = context
			.config()
			.chain
			.maptos_private_key_signer_identifier
			.try_raw_private_key()?;
		let private_key = Ed25519PrivateKey::try_from(raw_private_key.as_slice())?;
		let root_account = LocalAccount::new(
			aptos_test_root_address(),
			AccountKey::from_private_key(private_key),
			0,
		);

		// Seed for random number generator, used here to generate predictable results in a test environment.
		let seed = [3u8; 32];
		let mut rng = ::rand::rngs::StdRng::from_seed(seed);

		// Loop to simulate the execution of multiple blocks.
		for i in 0..10 {
			let (epoch, round) = executor.get_next_epoch_and_round()?;
			info!("Epoch: {}, Round: {}", epoch, round);

			// Generate a random block ID.
			let block_id = HashValue::random();
			// Clone the signer from the executor for signing the metadata.
			let signer = executor.signer.clone();
			// Get the current time in microseconds for the block timestamp.
			let current_time_microseconds = chrono::Utc::now().timestamp_micros() as u64;

			// Create a transaction factory with the chain ID of the executor, used for creating transactions.
			let tx_factory =
				TransactionFactory::new(context.config().chain.maptos_chain_id.clone())
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
			let (block_commitment, state) = executor.execute_block(block)?;
			info!("execution state:{state:?}");

			// Access the database reader to verify state after execution.
			let db_reader = executor.db_reader();
			// Get the latest version of the blockchain state from the database.
			let latest_version = db_reader.get_latest_ledger_info_version()?;

			info!("Latest version: {}", latest_version);
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
			assert_eq!(block_commitment.height(), i + 1);
			assert_eq!(block_commitment.commitment(), expected_commitment);
		}

		Ok(())
	}

	#[tokio::test]
	async fn test_execute_block_state_get_api() -> Result<(), anyhow::Error> {
		// Create an executor instance from the environment configuration.
		let private_key = Ed25519PrivateKey::generate_for_testing();

		let (tx_sender, _tx_receiver) = futures::channel::mpsc::channel::<MempoolClientRequest>(10);
		let (mempool_tx_exec_result_sender, mempool_commit_tx_receiver) =
			unbounded_channel::<Vec<TxExecutionResult>>();
		let (mut executor, _tempdir) =
			Executor::try_test_default(private_key, mempool_tx_exec_result_sender).await?;
		let (context, _transaction_pipe) =
			executor.background(mempool_commit_tx_receiver, tx_sender)?;
		let service = Service::new(&context);

		// Initialize a root account using a predefined keypair and the test root address.
		let raw_private_key = context
			.config()
			.chain
			.maptos_private_key_signer_identifier
			.try_raw_private_key()?;
		let private_key = Ed25519PrivateKey::try_from(raw_private_key.as_slice())?;
		let root_account = LocalAccount::new(
			aptos_test_root_address(),
			AccountKey::from_private_key(private_key),
			0,
		);

		// Seed for random number generator, used here to generate predictable results in a test environment.
		let seed = [3u8; 32];
		let mut rng = ::rand::rngs::StdRng::from_seed(seed);

		// Create a transaction factory with the chain ID of the executor.
		let tx_factory = TransactionFactory::new(context.config().chain.maptos_chain_id.clone());

		// Simulate the execution of multiple blocks.
		for _ in 0..10 {
			// For example, create and execute 3 blocks.
			let (epoch, round) = executor.get_next_epoch_and_round()?;

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
			executor.execute_block(block)?;

			// Retrieve the executor's API interface and fetch the transaction by each hash.
			let apis = service.get_apis();
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
	async fn test_revert_block_head_to() -> Result<(), anyhow::Error> {
		Ok(())
	}
}
