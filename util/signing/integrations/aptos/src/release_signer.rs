use crate::TransactionSigner;
use maptos_framework_release_util::{ReleaseSigner, ReleaseSignerError};

/// Wrapper around a [TransactionSigner] used to implement the [ReleaseSigner] trait.
pub struct TransactionReleaseSigner<T>(T)
where
	T: TransactionSigner + Sync;

impl<T> TransactionReleaseSigner<T>
where
	T: TransactionSigner + Sync,
{
	pub fn new(signer: T) -> Self {
		Self(signer)
	}

	pub fn as_inner(&self) -> &T {
		&self.0
	}
}

impl<T> ReleaseSigner for TransactionReleaseSigner<T>
where
	T: TransactionSigner + Sync,
{
	async fn sign_release(
		&self,
		raw_transaction: aptos_types::transaction::RawTransaction,
	) -> Result<aptos_types::transaction::SignedTransaction, ReleaseSignerError> {
		self.0
			.sign_transaction(raw_transaction)
			.await
			.map_err(|e| ReleaseSignerError::Signing(format!("{:?}", e).into()))
	}

	async fn release_account_authentication_key(
		&self,
	) -> Result<aptos_types::transaction::authenticator::AuthenticationKey, ReleaseSignerError> {
		self.0
			.authentication_key()
			.await
			.map_err(|e| ReleaseSignerError::Signing(format!("{:?}", e).into()))
	}
}
