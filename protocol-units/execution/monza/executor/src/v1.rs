use crate::*;
use aptos_types::transaction::SignedTransaction;
use async_channel::Sender;
use monza_opt_executor::Executor;
use async_channel::Sender;
use aptos_types::transaction::SignedTransaction;

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
        mode : &FinalityMode, 
        block: ExecutableBlock,
    ) -> Result<(), anyhow::Error> {

        match mode {
            FinalityMode::Dyn => unimplemented!(),
            FinalityMode::Opt => {
                println!("Executing opt block: {:?}", block.block_id);
                self.executor.execute_block(block).await
            },
            FinalityMode::Fin => unimplemented!(),
        }

    }

	/// Sets the transaction channel.
	async fn set_tx_channel(
		&mut self,
		tx_channel: Sender<SignedTransaction>,
	) -> Result<(), anyhow::Error> {
		self.transaction_channel = tx_channel;
		Ok(())
	}

	/// Gets the API.
	async fn get_api(&self, _mode: &FinalityMode) -> Result<Apis, anyhow::Error> {
		match _mode {
			FinalityMode::Dyn => unimplemented!(),
			FinalityMode::Opt => Ok(self.executor.try_get_apis().await?),
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
mod opt_tests {

	use super::*;
	use aptos_api::{accept_type::AcceptType, transactions::SubmitTransactionPost};
	use aptos_crypto::{
		ed25519::{Ed25519PrivateKey, Ed25519Signature},
		HashValue, PrivateKey, Uniform,
	};
	use aptos_mempool::{MempoolClientRequest, MempoolClientSender};
	use aptos_sdk::{transaction_builder::TransactionFactory, types::{AccountKey, LocalAccount}};
use aptos_storage_interface::state_view::DbStateViewAtVersion;
use aptos_types::{
		account_address::AccountAddress, account_config::aptos_test_root_address, account_view::AccountView, block_executor::partitioner::ExecutableTransactions, chain_id::ChainId, state_store::account_with_state_view::AsAccountWithStateView, transaction::{
			signature_verified_transaction::{into_signature_verified_block, SignatureVerifiedTransaction}, RawTransaction, Script,
			SignedTransaction, Transaction, TransactionPayload,
		}
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

		let request = SubmitTransactionPost::Bcs(aptos_api::bcs_payload::Bcs(bcs_user_transaction));
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

		let request = SubmitTransactionPost::Bcs(aptos_api::bcs_payload::Bcs(bcs_user_transaction));
		let api = executor.get_api(&FinalityMode::Opt).await?;
		api.transactions.submit_transaction(AcceptType::Bcs, request).await?;

		let received_transaction = rx.recv().await?;
		assert_eq!(received_transaction, comparison_user_transaction);

		// Now execute the block
		let block_id = HashValue::random();
		let tx =
			SignatureVerifiedTransaction::Valid(Transaction::UserTransaction(received_transaction));
		let txs = ExecutableTransactions::Unsharded(vec![tx]);
		let block = ExecutableBlock::new(block_id.clone(), txs);
		executor.execute_block(&FinalityMode::Opt, block).await?;

		services_handle.abort();
		background_handle.abort();

		Ok(())
	}

	#[tokio::test]
	async fn test_revert_chain_state() -> Result<(), anyhow::Error> {
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

		for _ in 0..10 {
			let user_transaction = create_signed_transaction(0);
			let comparison_user_transaction = user_transaction.clone();
			let bcs_user_transaction = bcs::to_bytes(&user_transaction)?;

			let request =
				SubmitTransactionPost::Bcs(aptos_api::bcs_payload::Bcs(bcs_user_transaction));
			let api = executor.get_api(&FinalityMode::Opt).await?;
			api.transactions.submit_transaction(AcceptType::Bcs, request).await?;

			let received_transaction = rx.recv().await?;
			assert_eq!(received_transaction, comparison_user_transaction);

            // Now execute the block
		    let block_id = HashValue::random();
		    let tx =
			    SignatureVerifiedTransaction::Valid(Transaction::UserTransaction(received_transaction));
		    let txs = ExecutableTransactions::Unsharded(vec![tx]);
		    let block = ExecutableBlock::new(block_id.clone(), txs);
		    executor.execute_block(&FinalityMode::Opt, block).await?;
		}

		let db = executor.executor.db.clone();
		services_handle.abort();
		background_handle.abort();

		Ok(())
	}

	// https://github.com/movementlabsxyz/aptos-core/blob/ea91067b81f9673547417bff9c70d5a2fe1b0e7b/execution/executor-test-helpers/src/integration_test_impl.rs#L535
	#[tokio::test]
	async fn test_execute_block_state_db() -> Result<(), anyhow::Error> {
		// Initialize a root account using a predefined keypair and the test root address.
		let root_account = LocalAccount::new(
			aptos_test_root_address(),
			AccountKey::from_private_key(aptos_vm_genesis::GENESIS_KEYPAIR.0.clone()),
			0,
		);

		// Seed for random number generator, used here to generate predictable results in a test environment.
		let seed = [3u8; 32];
		let mut rng = ::rand::rngs::StdRng::from_seed(seed);

		// Create an executor instance from the environment configuration.
		let executor = Executor::try_from_env()?;
		// Create a transaction factory with the chain ID of the executor, used for creating transactions.
		let tx_factory = TransactionFactory::new(executor.chain_id.clone());

		// Loop to simulate the execution of multiple blocks.
		for _ in 0..10 {
			// Generate a random block ID.
			let block_id = HashValue::random();
			// Clone the signer from the executor for signing the metadata.
			// let signer = executor.signer.clone();
			// Get the current time in microseconds for the block timestamp.
			// let current_time_micros = chrono::Utc::now().timestamp_micros() as u64;

			// Create a block metadata transaction.
			/*let block_metadata = Transaction::BlockMetadata(BlockMetadata::new(
				block_id,
				1,
				0,
				signer.author(),
				vec![0],
				vec![],
				current_time_micros,
			));*/

			// Create a state checkpoint transaction using the block ID.
			// let state_checkpoint_tx = Transaction::StateCheckpoint(block_id.clone());
			// Generate a new account for transaction tests.
			let new_account = LocalAccount::generate(&mut rng);
			let new_account_address = new_account.address();

			// Create a user account creation transaction.
			let user_account_creation_tx = root_account
				.sign_with_transaction_builder(tx_factory.create_user_account(new_account.public_key()));

			// Create a mint transaction to provide the new account with some initial balance.
			let mint_tx = root_account
				.sign_with_transaction_builder(tx_factory.mint(new_account.address(), 2000));
			// Store the hash of the committed transaction for later verification.
			let mint_tx_hash = mint_tx.clone().committed_hash();

			// Group all transactions into a single unsharded block for execution.
			let transactions = ExecutableTransactions::Unsharded(
				into_signature_verified_block(vec![
					// block_metadata,
					Transaction::UserTransaction(user_account_creation_tx),
					Transaction::UserTransaction(mint_tx),
					// state_checkpoint_tx,
				])
			);

			// Create and execute the block.
			let block = ExecutableBlock::new(block_id.clone(), transactions);
			executor.execute_block(block).await?;

			// Access the database reader to verify state after execution.
			let db_reader = executor.db.read().await.reader.clone();
			// Get the latest version of the blockchain state from the database.
			let latest_version = db_reader.get_latest_version()?;
			// Verify the transaction by its hash to ensure it was committed.
			let transaction_result = db_reader.get_transaction_by_hash(
				mint_tx_hash,
				latest_version,
				false,
			)?;
			assert!(transaction_result.is_some());

			// Create a state view at the latest version to inspect account states.
			let state_view = db_reader.state_view_at_version(Some(latest_version))?;
			// Access the state view of the new account to verify its state and existence.
			let account_state_view = state_view.as_account_with_state_view(&new_account_address);
			let queried_account_address = account_state_view.get_account_address()?;
			assert!(queried_account_address.is_some());
			let account_resource = account_state_view.get_account_resource()?;
			assert!(account_resource.is_some());
		}

		Ok(())
	}

}
