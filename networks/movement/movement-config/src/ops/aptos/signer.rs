use crate::Config;
use movement_signer::cryptography::ed25519::Ed25519;
use movement_signer_loader::{Load, LoadedSigner};
use std::future::Future;

/// Errors thrown when attempting to use the config for an Aptos rest client.
#[derive(Debug, thiserror::Error)]
pub enum TransactionSignerOperationsError {
	#[error("building transaction signer failed: {0}")]
	BuildingTransactionSignerOperations(#[source] Box<dyn std::error::Error + Send + Sync>),
}

/// A trait for [TransactionSignerOperations] operations.
///
/// This is useful for managing imports and adding sub implementations.
pub trait TransactionSignerOperations {
	fn get_transaction_signer(
		&self,
	) -> impl Future<Output = Result<LoadedSigner<Ed25519>, TransactionSignerOperationsError>>;
}

impl TransactionSignerOperations for Config {
	async fn get_transaction_signer(
		&self,
	) -> Result<LoadedSigner<Ed25519>, TransactionSignerOperationsError> {
		// get the relevant fields from the config
		let signer = self
			.execution_config
			.maptos_config
			.chain
			.maptos_private_key_signer_identifier
			.load()
			.await
			.map_err(|e| {
				TransactionSignerOperationsError::BuildingTransactionSignerOperations(
					format!("failed to load signer identifier: {}", e).into(),
				)
			})?;

		Ok(signer)
	}
}
