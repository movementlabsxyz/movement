use crate::{Error, Verified, VerifierOperations};
use celestia_rpc::Client;
use celestia_types::{nmt::Namespace, Blob};
use movement_celestia_da_util::ir_blob::IntermediateBlobRepresentation;
use std::sync::Arc;

#[derive(Clone)]
pub struct Verifier {
	/// The Celestia RPC client
	pub client: Arc<Client>,
	/// The namespace of the Celestia Blob
	pub namespace: Namespace,
}

impl Verifier {
	pub fn new(client: Arc<Client>, namespace: Namespace) -> Self {
		Self { client, namespace }
	}
}

#[tonic::async_trait]
impl VerifierOperations<Blob, IntermediateBlobRepresentation> for Verifier {
	/// Verifies a Celestia Blob as a Valid IntermediateBlobRepresentation
	async fn verify(
		&self,
		blob: Blob,
		_height: u64,
	) -> Result<Verified<IntermediateBlobRepresentation>, Error> {
		// Only assert that we can indeed get an IntermediateBlobRepresentation from the Blob
		let ir_blob = IntermediateBlobRepresentation::try_from(blob)
			.map_err(|e| Error::Internal(e.to_string()))?;

		Ok(Verified::new(ir_blob))
	}
}

pub mod pessimistic;
#[cfg(all(test, feature = "integration-tests"))]
mod tests {
	use super::*;
	use celestia_types::blob::GasPrice;

	/// todo: Investigate why this test sporadically fails.
	#[tokio::test]
	pub async fn test_valid_verifies() -> Result<(), anyhow::Error> {
		let dot_movement = dot_movement::DotMovement::try_from_env()?;
		let config = dot_movement
			.try_get_config_from_json::<movement_celestia_da_util::CelestiaDaLightNodeConfig>()?;

		let data = vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
		let blob = Blob::new(celestia_namespace.clone(), data.clone())?;

		let height = client.blob_submit(&[blob], GasPrice::default()).await?;

		let included = verifier.verify(VerificationMode::Cowboy, &data, height).await?;

		assert!(included);

		Ok(())
	}

	#[tokio::test]
	pub async fn test_absent_does_not_verify() -> Result<(), anyhow::Error> {
		let dot_movement = dot_movement::DotMovement::try_from_env()?;
		let config = dot_movement
			.try_get_config_from_json::<movement_celestia_da_util::CelestiaDaLightNodeConfig>()?;
		let client = Arc::new(config.connect_celestia().await?);
		let celestia_namespace = config.celestia_namespace();

		let verifier = Verifier { client: client.clone(), namespace: celestia_namespace.clone() };

		let data = vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
		let blob = Blob::new(celestia_namespace.clone(), data.clone())?;

		let height = client.blob_submit(&[blob], GasPrice::default()).await?;

		let included = verifier.verify(VerificationMode::Cowboy, &data, height).await?;

		assert!(included);

		let absent_data = vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 7];

		let absent_included = verifier.verify(VerificationMode::Cowboy, &absent_data, height).await;

		match absent_included {
			Ok(_) => {
				assert!(false, "Should not have verified")
			}
			Err(_) => {}
		}

		Ok(())
	}

	#[tokio::test]
	pub async fn test_wrong_height_does_not_verify() -> Result<(), anyhow::Error> {
		let dot_movement = dot_movement::DotMovement::try_from_env()?;
		let config = dot_movement
			.try_get_config_from_json::<movement_celestia_da_util::CelestiaDaLightNodeConfig>()?;
		let client = Arc::new(config.connect_celestia().await?);
		let celestia_namespace = config.celestia_namespace();

		let verifier = Verifier { client: client.clone(), namespace: celestia_namespace.clone() };

		let data = vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
		let blob = Blob::new(celestia_namespace.clone(), data.clone())?;

		let height = client.blob_submit(&[blob], GasPrice::default()).await?;

		let included = verifier.verify(VerificationMode::Cowboy, &data, height).await?;

		assert!(included);

		let wrong_height_included =
			verifier.verify(VerificationMode::Cowboy, &data, height + 1).await;

		match wrong_height_included {
			Ok(_) => {
				assert!(false, "Should not have verified")
			}
			Err(_) => {}
		}

		Ok(())
	}
}
