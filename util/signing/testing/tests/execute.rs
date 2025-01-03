use maptos_dof_execution::{v1::Executor, DynOptFinExecutor};
use maptos_dof_execution::{ExecutableBlock, ExecutableTransactions, SignatureVerifiedTransaction};
use maptos_execution_util::config::Config;
use movement_signer_test::ed25519::TestSigner;
use movement_signing_aptos::TransactionSigner;

use aptos_crypto::{ed25519::Ed25519PrivateKey, HashValue, Uniform};
use aptos_types::account_address::AccountAddress;
use aptos_types::chain_id::ChainId;
use aptos_types::transaction::{
	RawTransaction, Script, SignedTransaction, Transaction, TransactionPayload,
};

use anyhow::Context;
use tempfile::TempDir;

fn setup(mut maptos_config: Config) -> Result<(Executor, TempDir), anyhow::Error> {
	let tempdir = tempfile::tempdir()?;
	// replace the db path with the temporary directory
	maptos_config.chain.maptos_db_path.replace(tempdir.path().to_path_buf());
	let executor = Executor::try_from_config(maptos_config)?;
	Ok((executor, tempdir))
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
	config.chain.maptos_private_key = private_key.clone();
	let signer = TestSigner::new(signing_key);
	let (executor, _tempdir) = setup(config)?;
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
	executor.execute_block_opt(block).await?;
	Ok(())
}
