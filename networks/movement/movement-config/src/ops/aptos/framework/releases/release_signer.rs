use crate::{ops::aptos::signer::TransactionSignerOperations, Config};
use movement_signer::cryptography::ed25519::Ed25519;
use movement_signer_loader::LoadedSigner;
use movement_signing_aptos::release_signer::TransactionReleaseSigner;
use std::future::Future;

/// Errors thrown when attempting to use the config for an Aptos rest client.
#[derive(Debug, thiserror::Error)]
pub enum ReleaseSignerOperationsError {
	#[error("building release signer failed: {0}")]
	BuildingReleaseSigner(#[source] Box<dyn std::error::Error + Send + Sync>),
}

/// A trait for [ReleaseSignerOperations] operations.
///
/// This is useful for managing imports and adding sub implementations.
pub trait ReleaseSignerOperations {
	fn get_release_signer(
		&self,
	) -> impl Future<
		Output = Result<
			TransactionReleaseSigner<LoadedSigner<Ed25519>>,
			ReleaseSignerOperationsError,
		>,
	>;
}

impl ReleaseSignerOperations for Config {
	async fn get_release_signer(
		&self,
	) -> Result<TransactionReleaseSigner<LoadedSigner<Ed25519>>, ReleaseSignerOperationsError> {
		// get the transaction signer
		let loaded_signer = self.get_transaction_signer().await.map_err(|e| {
			ReleaseSignerOperationsError::BuildingReleaseSigner(
				format!("failed to get transaction signer: {}", e).into(),
			)
		})?;

		// build the release signer
		let release_signer = TransactionReleaseSigner::new(loaded_signer);

		Ok(release_signer)
	}
}
