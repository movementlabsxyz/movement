use aptos_crypto::ed25519::{Ed25519PublicKey, Ed25519Signature};
use aptos_crypto::CryptoMaterialError;
use aptos_types::transaction::{RawTransaction, SignedTransaction};
use movement_signer::{cryptography::ed25519::Ed25519, Signer, SignerError};

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
	T: Signer<Ed25519> + Sync,
{
	async fn sign_transaction_bytes(&self, bytes: &[u8]) -> Result<Ed25519Signature, Error> {
		let signature = self.sign(bytes).await?;
		let signature = signature.as_bytes().try_into()?;
		Ok(signature)
	}

	async fn public_key(&self) -> Result<Ed25519PublicKey, Error> {
		let key = <Self as Signer<Ed25519>>::public_key(self).await?;
		let key = key.as_bytes().try_into()?;
		Ok(key)
	}
}
