pub mod key_rotation;
pub mod release_signer;

use aptos_crypto::ed25519::{Ed25519PublicKey, Ed25519Signature};
use aptos_crypto::CryptoMaterialError;
use aptos_types::transaction::{
	authenticator::AuthenticationKey, RawTransaction, SignedTransaction,
};
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
	/// Signs a raw transaction and returns a signed transaction.
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

	/// Signs a message and returns a signature.
	fn sign_transaction_bytes(
		&self,
		bytes: &[u8],
	) -> impl Future<Output = Result<Ed25519Signature, Error>> + Send;

	/// Returns the public key of the signer.
	fn public_key(&self) -> impl Future<Output = Result<Ed25519PublicKey, Error>> + Send;

	/// Returns the authentication key of the signer.
	fn authentication_key(&self) -> impl Future<Output = Result<AuthenticationKey, Error>> + Send {
		async move {
			let public_key = self.public_key().await?;
			Ok(AuthenticationKey::ed25519(&public_key))
		}
	}
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
	use aptos_types::account_address::AccountAddress;
	use aptos_types::chain_id::ChainId;
	use aptos_types::transaction::{Script, TransactionPayload};
	use movement_signer_hashicorp_vault::hsm::HashiCorpVault;

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
}
