use crate::{
	ops::aptos::{
		rotate_key::core_resource_account::CoreResourceAccountKeyRotationSigner,
		signer::TransactionSignerOperations,
	},
	Config,
};
use movement_signer::cryptography::ed25519::Ed25519;
use movement_signer_loader::identifiers::SignerIdentifier;
use movement_signer_loader::LoadedSigner;
use movement_signing_aptos::key_rotation::signer::{
	KeyRotationSigner, TransactionKeyRotationSigner,
};
use std::future::Future;

/// Errors thrown when attempting to use the config for an Aptos rest client.
#[derive(Debug, thiserror::Error)]
pub enum LoadKeyRotationSignerError {
	#[error("building key rotation signer failed: {0}")]
	BuildingKeyRotationSigner(#[source] Box<dyn std::error::Error + Send + Sync>),
}

pub struct Signer(TransactionKeyRotationSigner<LoadedSigner<Ed25519>>);

impl Signer {
	pub fn new(signer: TransactionKeyRotationSigner<LoadedSigner<Ed25519>>) -> Self {
		Self(signer)
	}
}

impl KeyRotationSigner for Signer {
	fn sign_key_rotation(
		&self,
		raw_transaction: aptos_sdk::types::transaction::RawTransaction,
	) -> impl Future<
		Output = Result<
			aptos_sdk::types::transaction::SignedTransaction,
			movement_signing_aptos::key_rotation::signer::KeyRotationSignerError,
		>,
	> {
		self.0.sign_key_rotation(raw_transaction)
	}

	fn public_key(
		&self,
	) -> impl Future<
		Output = Result<
			aptos_sdk::crypto::ed25519::PublicKey,
			movement_signing_aptos::key_rotation::signer::KeyRotationSignerError,
		>,
	> {
		self.0.public_key()
	}

	fn sign_message(
		&self,
		message: &[u8],
	) -> impl Future<
		Output = Result<
			aptos_sdk::crypto::ed25519::Signature,
			movement_signing_aptos::key_rotation::signer::KeyRotationSignerError,
		>,
	> {
		self.0.sign_message(message)
	}

	fn key_rotation_account_address(
		&self,
		client: &aptos_sdk::rest_client::Client,
	) -> impl Future<
		Output = Result<
			aptos_sdk::types::PeerId,
			movement_signing_aptos::key_rotation::signer::KeyRotationSignerError,
		>,
	> {
		self.0.key_rotation_account_address(client)
	}

	fn key_rotation_account_sequence_number(
		&self,
		client: &aptos_sdk::rest_client::Client,
	) -> impl Future<
		Output = Result<u64, movement_signing_aptos::key_rotation::signer::KeyRotationSignerError>,
	> {
		self.0.key_rotation_account_sequence_number(client)
	}

	fn key_rotation_account_authentication_key(
		&self,
	) -> impl Future<
		Output = Result<
			aptos_sdk::types::transaction::authenticator::AuthenticationKey,
			movement_signing_aptos::key_rotation::signer::KeyRotationSignerError,
		>,
	> {
		// use the inner method
		self.0.key_rotation_account_authentication_key()
	}
}

impl CoreResourceAccountKeyRotationSigner for Signer {
	fn signer_identifier(&self) -> SignerIdentifier {
		self.0.as_inner().identifier().clone()
	}
}

/// A trait for [LoadKeyRotationSigner] operations.
///
/// This is useful for managing imports and adding sub implementations.
pub trait LoadKeyRotationSigner {
	fn load_key_rotation_signer(
		&self,
	) -> impl Future<Output = Result<Signer, LoadKeyRotationSignerError>>;
}

impl LoadKeyRotationSigner for Config {
	async fn load_key_rotation_signer(&self) -> Result<Signer, LoadKeyRotationSignerError> {
		// load the transaction signer
		let loaded_signer = self.get_transaction_signer().await.map_err(|e| {
			LoadKeyRotationSignerError::BuildingKeyRotationSigner(
				format!("failed to load transaction signer: {}", e).into(),
			)
		})?;

		// build the release signer
		let key_rotation_signer = TransactionKeyRotationSigner::new(loaded_signer);

		// wrap the signer
		let key_rotation_signer = Signer::new(key_rotation_signer);

		Ok(key_rotation_signer)
	}
}
