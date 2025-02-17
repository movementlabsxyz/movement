use crate::TransactionSigner;
use aptos_crypto::ed25519::{Ed25519PublicKey, Ed25519Signature};
use aptos_sdk::{
	rest_client::Client,
	types::{
		account_address::AccountAddress,
		transaction::authenticator::AuthenticationKey,
		transaction::{RawTransaction, SignedTransaction},
	},
};
use movement::account::key_rotation::lookup_address;
use std::error;
use std::future::Future;

#[derive(Debug, thiserror::Error)]
pub enum KeyRotationSignerError {
	#[error("signing key rotation failed with: {0}")]
	Signing(#[source] Box<dyn error::Error + Send + Sync>),
	#[error("account address for key rotation not found: {0}")]
	AccountAddressNotFound(#[source] Box<dyn error::Error + Send + Sync>),
}

pub trait KeyRotationSigner {
	/// Signs the given raw transaction.
	fn sign_key_rotation(
		&self,
		raw_transaction: RawTransaction,
	) -> impl Future<Output = Result<SignedTransaction, KeyRotationSignerError>>;

	/// Gets the public key of the signer
	fn public_key(&self) -> impl Future<Output = Result<Ed25519PublicKey, KeyRotationSignerError>>;

	/// Gets the authentication key of the signer.
	fn key_rotation_account_authentication_key(
		&self,
	) -> impl Future<Output = Result<AuthenticationKey, KeyRotationSignerError>>;

	/// Associated method for getting the account address of the signer.
	fn default_key_rotation_account_address(
		&self,
		client: &Client,
	) -> impl Future<Output = Result<AccountAddress, KeyRotationSignerError>> {
		async move {
			// get the authentication key
			let authentication_key = self.key_rotation_account_authentication_key().await?;

			// form the lookup address from the authentication key
			let lookup = AccountAddress::new(*authentication_key.account_address());

			// lookup the account address
			let account_address = lookup_address(client, lookup, true)
				.await
				.map_err(|e| KeyRotationSignerError::AccountAddressNotFound(Box::new(e)))?;

			Ok(account_address)
		}
	}

	/// Gets the account address of the signer.
	fn key_rotation_account_address(
		&self,
		client: &Client,
	) -> impl Future<Output = Result<AccountAddress, KeyRotationSignerError>> {
		async move { self.default_key_rotation_account_address(client).await }
	}

	/// Get the key_rotation account sequence number.
	fn key_rotation_account_sequence_number(
		&self,
		client: &Client,
	) -> impl Future<Output = Result<u64, KeyRotationSignerError>> {
		async move {
			let account_address = self.key_rotation_account_address(client).await?;
			let account = client
				.get_account(account_address)
				.await
				.map_err(|e| KeyRotationSignerError::AccountAddressNotFound(Box::new(e)))?;
			Ok(account.into_inner().sequence_number)
		}
	}

	/// Signs an arbitrary message.
	fn sign_message(
		&self,
		message: &[u8],
	) -> impl Future<Output = Result<Ed25519Signature, KeyRotationSignerError>>;
}

/// Wrapper around a [TransactionSigner] used to implement the [KeyRotationSigner] trait.
pub struct TransactionKeyRotationSigner<T>(T)
where
	T: TransactionSigner + Sync;

impl<T> TransactionKeyRotationSigner<T>
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

impl<T> KeyRotationSigner for TransactionKeyRotationSigner<T>
where
	T: TransactionSigner + Sync,
{
	async fn sign_key_rotation(
		&self,
		raw_transaction: aptos_types::transaction::RawTransaction,
	) -> Result<aptos_types::transaction::SignedTransaction, KeyRotationSignerError> {
		self.0
			.sign_transaction(raw_transaction)
			.await
			.map_err(|e| KeyRotationSignerError::Signing(format!("{:?}", e).into()))
	}

	async fn sign_message(
		&self,
		message: &[u8],
	) -> Result<Ed25519Signature, KeyRotationSignerError> {
		// we'll simply use [TransactionSigner::sign_transaction_bytes] to sign the message
		self.0
			.sign_transaction_bytes(message)
			.await
			.map_err(|e| KeyRotationSignerError::Signing(format!("{:?}", e).into()))
	}

	async fn key_rotation_account_authentication_key(
		&self,
	) -> Result<aptos_types::transaction::authenticator::AuthenticationKey, KeyRotationSignerError>
	{
		self.0
			.authentication_key()
			.await
			.map_err(|e| KeyRotationSignerError::Signing(format!("{:?}", e).into()))
	}

	async fn public_key(&self) -> Result<Ed25519PublicKey, KeyRotationSignerError> {
		self.0
			.public_key()
			.await
			.map_err(|e| KeyRotationSignerError::Signing(format!("{:?}", e).into()))
	}
}
