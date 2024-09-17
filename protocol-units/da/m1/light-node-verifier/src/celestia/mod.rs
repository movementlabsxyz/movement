use crate::{Error, Verified, VerifierOperations};
use celestia_rpc::Client;
#[cfg(feature = "pessimistic")]
use celestia_rpc::{BlobClient, HeaderClient};
use celestia_types::{nmt::Namespace, Blob};
use m1_da_light_node_util::inner_blob::InnerBlob;
use std::sync::Arc;

#[derive(Clone)]
pub struct Verifier {
	pub client: Arc<Client>,
	pub namespace: Namespace,
}

impl Verifier {
	pub fn new(client: Arc<Client>, namespace: Namespace) -> Self {
		Self { client, namespace }
	}
}

#[tonic::async_trait]
impl VerifierOperations<Blob, InnerBlob> for Verifier {
	/// Verifies a Celestia Blob as a Valid InnerBlob
	async fn verify(&self, blob: Blob, _height: u64) -> Result<Verified<InnerBlob>, Error> {
		//@l-monninger: the light node itself does most of the work of verify blobs. The verification under the feature flag below is useful in zero-trust environments.
		#[cfg(feature = "pessimistic")]
		{
			blob.validate().map_err(|e| Error::Validation(e.to_string()))?;

			// wait for the header to be at the correct height
			self.client
				.header_wait_for_height(height)
				.await
				.map_err(|e| Error::Internal(e.to_string()))?;

			// get the root
			let dah = self
				.client
				.header_get_by_height(height)
				.await
				.map_err(|e| Error::Internal(e.to_string()))?
				.dah;
			let root_hash = dah.row_root(0).ok_or(Error::Validation("No root hash".to_string()))?;

			// get the proof
			let proofs = self
				.client
				.blob_get_proof(height, self.namespace.clone(), blob.commitment)
				.await
				.map_err(|e| Error::Internal(e.to_string()))?;

			// get the leaves
			let leaves = blob.to_shares().map_err(|e| Error::Internal(e.to_string()))?;

			// check if included
			for proof in proofs.iter() {
				proof
					.verify_complete_namespace(&root_hash, &leaves, self.namespace.into())
					.map_err(|_e| {
						Error::Validation("failed to verify complete namespace".to_string())
					})?;
			}
		}

		let inner_blob = InnerBlob::try_from(blob).map_err(|e| Error::Internal(e.to_string()))?;

		Ok(Verified::new(inner_blob))
	}
}

#[cfg(all(test, feature = "integration-tests"))]
mod tests {
	use super::*;
	use celestia_types::blob::GasPrice;

	/// todo: Investigate why this test sporadically fails.
	#[tokio::test]
	pub async fn test_valid_verifies() -> Result<(), anyhow::Error> {
		let dot_movement = dot_movement::DotMovement::try_from_env()?;
		let config = dot_movement
			.try_get_config_from_json::<m1_da_light_node_util::M1DaLightNodeConfig>()?;

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
			.try_get_config_from_json::<m1_da_light_node_util::M1DaLightNodeConfig>()?;
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
			.try_get_config_from_json::<m1_da_light_node_util::M1DaLightNodeConfig>()?;
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
