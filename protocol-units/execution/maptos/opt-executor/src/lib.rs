pub mod background;
pub mod bootstrap;
pub mod context;
#[warn(unused_imports)]
pub mod executor;
pub mod gc_account_sequence_number;
pub mod indexer;
pub mod service;

pub use context::Context;
pub use executor::Executor;
pub use service::Service;

#[cfg(test)]
mod tests {

	use crate::executor::{TxExecutionResult, EXECUTOR_CHANNEL_SIZE};
	use crate::Executor;
	use aptos_crypto::HashValue;
	use aptos_sdk::types::account_config::aptos_test_root_address;
	use aptos_sdk::types::account_config::AccountResource;
	use aptos_sdk::{transaction_builder::TransactionFactory, types::LocalAccount};
	use aptos_storage_interface::state_view::DbStateViewAtVersion;
	use aptos_types::state_store::MoveResourceExt;
	use aptos_types::{
		account_address::AccountAddress,
		block_executor::partitioner::{ExecutableBlock, ExecutableTransactions},
		block_metadata::BlockMetadata,
		chain_id::ChainId,
		transaction::signature_verified_transaction::{
			into_signature_verified_block, SignatureVerifiedTransaction,
		},
		transaction::{RawTransaction, Script, SignedTransaction, Transaction, TransactionPayload},
	};
	use movement_signer::cryptography::ed25519::Ed25519;
	use movement_signer_hashicorp_vault::hsm::HashiCorpVault;
	use movement_signing_aptos::TransactionSigner;
	use rand::SeedableRng;
	use tokio::sync::mpsc;

	#[tokio::test]
	#[ignore]
	async fn test_sign_transaction_with_hashi_corp_vault_includes_in_block(
	) -> Result<(), anyhow::Error> {
		dotenv::dotenv().ok();
		let hsm = HashiCorpVault::<Ed25519>::create_random_key().await?;
		let public_key = TransactionSigner::public_key(&hsm).await?;

		let transaction_payload = TransactionPayload::Script(Script::new(vec![0], vec![], vec![]));
		let raw_transaction = RawTransaction::new(
			AccountAddress::random(),
			0,
			transaction_payload,
			0,
			0,
			0,
			ChainId::test(),
		);
		let signed_transaction = TransactionSigner::sign_transaction(&hsm, raw_transaction).await?;

		let (tx_sender, _tx_receiver) = mpsc::channel::<Vec<(u64, SignedTransaction)>>(1);

		let (mempool_tx_exec_result_sender, mempool_commit_tx_receiver) =
			futures::channel::mpsc::channel::<Vec<TxExecutionResult>>(EXECUTOR_CHANNEL_SIZE);

		let (mut executor, _tempdir) =
			Executor::try_test_default_with_public_key(public_key, mempool_tx_exec_result_sender)?;
		let (_context, _transaction_pipe) = executor.background(mempool_commit_tx_receiver)?;
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
			signed_transaction.clone(),
		));
		let txs = ExecutableTransactions::Unsharded(vec![
			SignatureVerifiedTransaction::Valid(block_metadata),
			tx,
		]);
		let block = ExecutableBlock::new(block_id.clone(), txs);
		executor.execute_block(block).await?;

		Ok(())
	}

	#[tokio::test]
	#[tracing_test::traced_test]
	#[ignore]
	async fn test_sign_transaction_with_hashi_corp_vault_executes() -> Result<(), anyhow::Error> {
		dotenv::dotenv().ok();
		let hsm = HashiCorpVault::<Ed25519>::create_random_key().await?;
		let public_key = TransactionSigner::public_key(&hsm).await?;
		let account_address = aptos_test_root_address();

		let (tx_sender, _tx_receiver) = mpsc::channel::<Vec<(u64, SignedTransaction)>>(1);

		let (mempool_tx_exec_result_sender, mempool_commit_tx_receiver) =
			futures::channel::mpsc::channel::<Vec<TxExecutionResult>>(EXECUTOR_CHANNEL_SIZE);
		let (mut executor, _tempdir) =
			Executor::try_test_default_with_public_key(public_key, mempool_tx_exec_result_sender)?;
		let (context, _transaction_pipe) = executor.background(mempool_commit_tx_receiver)?;

		// Seed for random number generator, used here to generate predictable results in a test environment.
		let seed = [3u8; 32];
		let mut rng = ::rand::rngs::StdRng::from_seed(seed);

		// Create a transaction factory with the chain ID of the executor.
		let tx_factory = TransactionFactory::new(context.config().chain.maptos_chain_id.clone());

		// Generate a new account for transaction tests.
		let new_account = LocalAccount::generate(&mut rng);
		let new_account_address = new_account.address();

		// Create a new user account signing with the hsm.
		let raw_create_account_transaction = tx_factory
			.create_user_account(new_account.public_key())
			.sender(account_address)
			.sequence_number(0)
			.build();
		let signed_create_account_transaction =
			TransactionSigner::sign_transaction(&hsm, raw_create_account_transaction).await?;

		// Create a mint transaction to fund the new account.
		// Create a mint transaction to provide the new account with some initial balance.
		let mint_transaction = tx_factory
			.mint(new_account.address(), 2000)
			.sender(account_address)
			.sequence_number(1)
			.build();
		let signed_mint_transaction =
			TransactionSigner::sign_transaction(&hsm, mint_transaction).await?;
		let mint_transaction_hash = signed_mint_transaction.committed_hash();

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
		// Block Metadata
		let transactions = ExecutableTransactions::Unsharded(into_signature_verified_block(vec![
			block_metadata,
			Transaction::UserTransaction(signed_create_account_transaction),
			Transaction::UserTransaction(signed_mint_transaction),
		]));
		let block = ExecutableBlock::new(block_id.clone(), transactions);
		executor.execute_block(block).await?;

		// Access the database reader to verify state after execution.
		let db_reader = executor.db_reader();
		// Get the latest version of the blockchain state from the database.
		let latest_version = db_reader.get_synced_version()?;
		// Verify the transaction by its hash to ensure it was committed.
		let transaction_result =
			db_reader.get_transaction_by_hash(mint_transaction_hash, latest_version, false)?;
		assert!(transaction_result.is_some());

		// Create a state view at the latest version to inspect account states.
		let state_view = db_reader.state_view_at_version(Some(latest_version))?;
		// Access the state view of the new account to verify its state and existence.
		let _account_resource =
			AccountResource::fetch_move_resource(&state_view, &new_account_address)?.unwrap();

		Ok(())
	}
}
