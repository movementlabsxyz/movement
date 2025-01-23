use aptos_crypto::ed25519::{Ed25519PublicKey, Ed25519Signature};
use aptos_crypto::CryptoMaterialError;
use aptos_types::transaction::{RawTransaction, SignedTransaction};
use movement_signer::{cryptography::ed25519::Ed25519, SignerError, Signing};

use std::future::Future;

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
	#[error("failed to produce signing material")]
	CryptoMaterial(#[from] CryptoMaterialError),
	#[error(transparent)]
	Signer(#[from] SignerError),
}

pub trait TransactionSigner: Sync {
	fn sign_transaction(
		&self,
		raw: RawTransaction,
	) -> impl Future<Output = Result<SignedTransaction, Error>> + Send {
		async move {
			let message = aptos_crypto::signing_message(&raw)?;
			let signature = self.sign_transaction_bytes(&message).await?;
			let public_key = self.public_key().await?;
			Ok(SignedTransaction::new(raw, public_key, signature))
		}
	}

	fn sign_transaction_bytes(
		&self,
		bytes: &[u8],
	) -> impl Future<Output = Result<Ed25519Signature, Error>> + Send;

	fn public_key(&self) -> impl Future<Output = Result<Ed25519PublicKey, Error>> + Send;
}

impl<T> TransactionSigner for T
where
	T: Signing<Ed25519> + Sync,
{
	async fn sign_transaction_bytes(&self, bytes: &[u8]) -> Result<Ed25519Signature, Error> {
		let signature = self.sign(bytes).await?;
		let signature = signature.as_bytes().try_into()?;
		Ok(signature)
	}

	async fn public_key(&self) -> Result<Ed25519PublicKey, Error> {
		let key = <Self as Signing<Ed25519>>::public_key(self).await?;
		let key = key.as_bytes().try_into()?;
		Ok(key)
	}
}

#[cfg(test)]
pub mod test {

	use super::*;
	use aptos_crypto::HashValue;
	use aptos_types::{
		account_address::AccountAddress,
		block_executor::partitioner::{ExecutableBlock, ExecutableTransactions},
		block_metadata::BlockMetadata,
		chain_id::ChainId,
		transaction::signature_verified_transaction::SignatureVerifiedTransaction,
		transaction::{RawTransaction, Script, Transaction, TransactionPayload},
	};
	use maptos_opt_executor::Executor;
	use movement_signer_hashicorp_vault::hsm::HashiCorpVault;
	use tokio::sync::mpsc;

	#[tokio::test]
	async fn test_sign_transaction_with_hashi_corp_vault_verifies() -> Result<(), anyhow::Error> {
		dotenv::dotenv().ok();
		let hsm = HashiCorpVault::<Ed25519>::create_random_key().await?;

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
		signed_transaction.verify_signature().map_err(|e| anyhow::anyhow!(e))?;
		Ok(())
	}

	#[tokio::test]
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

		let (tx_sender, _tx_receiver) = mpsc::channel(1);
		let (executor, _tempdir) = Executor::try_test_default_with_public_key(public_key)?;
		let (context, _transaction_pipe) = executor.background(tx_sender)?;
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
}
