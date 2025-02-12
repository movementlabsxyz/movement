pub mod dot_movement;
pub mod load_key_rotation_signer;
pub mod signer;

use crate::Config;
use movement_signing_aptos::key_rotation::KeyRotator;
use signer::CoreResourceAccountKeyRotationSigner;
use std::future::Future;

/// A helper struct to rotate the core resource account key.
pub struct RotateCoreResourceAccountKey;

impl RotateCoreResourceAccountKey {
	/// Creates a new instance of `RotateCoreResourceAccountKey`.
	pub fn new() -> Self {
		Self
	}

	/// Rotates the core resource account key and updates the config.
	pub async fn rotate_core_resource_account_key(
		&self,
		old_config: Config,
		client: &aptos_sdk::rest_client::Client,
		old_signer: &impl CoreResourceAccountKeyRotationSigner,
		new_signer: &impl CoreResourceAccountKeyRotationSigner,
	) -> Result<Config, RotateCoreResourceAccountError> {
		// use the normal key rotator
		let key_rotator = KeyRotator::new();

		// rotate the key
		key_rotator
			.rotate_key_1pc(client, old_signer, new_signer)
			.await
			.map_err(|e| RotateCoreResourceAccountError::KeyRotationFailed(e.into()))?;

		// get the identifier for the new signer
		let new_key = new_signer.signer_identifier();

		// update the config
		let mut config = old_config;
		config.execution_config.maptos_config.chain.maptos_private_key_signer_identifier = new_key;

		Ok(config)
	}
}

/// Errors thrown by RotateCoreResourceAccount migrations.
#[derive(Debug, thiserror::Error)]
pub enum RotateCoreResourceAccountError {
	#[error("key rotation failed: {0}")]
	KeyRotationFailed(#[source] Box<dyn std::error::Error + Send + Sync>),
}

pub trait RotateCoreResourceAccountKeyOperations {
	/// Handles all side effects of rotating the core resource account key including writing to file and outputs a copy of the updated config.
	fn rotate_core_resource_account_key(
		&self,
		new_signer: &impl CoreResourceAccountKeyRotationSigner,
	) -> impl Future<Output = Result<Config, RotateCoreResourceAccountError>>;
}
