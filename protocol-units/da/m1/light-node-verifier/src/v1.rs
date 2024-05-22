use crate::{ToVerifierError, Verifier, VerifierError};
use celestia_rpc::{BlobClient, Client, HeaderClient};
use celestia_types::{nmt::Namespace, Blob};
use m1_da_light_node_grpc::VerificationMode;
use std::sync::Arc;

#[derive(Clone)]
pub struct V1Verifier {
	pub client: Arc<Client>,
	pub namespace: Namespace,
}

#[tonic::async_trait]
impl Verifier for V1Verifier {
	/// All verification is the same for now
	async fn verify(
		&self,
		_verification_mode: VerificationMode,
		blob: &[u8],
		height: u64,
	) -> Result<bool, VerifierError> {
		let celestia_blob = Blob::new(self.namespace.clone(), blob.to_vec()).map_verifier_err()?;

		celestia_blob.validate().map_verifier_err()?;

		// wait for the header to be at the correct height
		self.client.header_wait_for_height(height).await.map_verifier_err()?;

		// get the root
		let dah = self.client.header_get_by_height(height).await.map_verifier_err()?.dah;
		let root_hash = dah.row_root(0).ok_or(VerifierError::MissingRootHash)?;

		// get the proof
		let proofs = self
			.client
			.blob_get_proof(height, self.namespace.clone(), celestia_blob.commitment)
			.await
			.map_verifier_err()?;

		// get the leaves
		let leaves = celestia_blob.to_shares().map_verifier_err()?;

		// check if included
		for proof in proofs.iter() {
			proof
				.verify_complete_namespace(&root_hash, &leaves, self.namespace.into())
				.map_err(|_| VerifierError::VerifyProofFailed)?;
		}

		Ok(true)
	}

	async fn verify_cowboy(
		&self,
		_verification_mode: VerificationMode,
		_blob: &[u8],
		_height: u64,
	) -> Result<bool, VerifierError> {
		unimplemented!()
	}

	async fn verify_m_of_n(
		&self,
		_verification_mode: VerificationMode,
		_blob: &[u8],
		_height: u64,
	) -> Result<bool, VerifierError> {
		unimplemented!()
	}

	async fn verifiy_validator_in(
		&self,
		_verification_mode: VerificationMode,
		_blob: &[u8],
		_height: u64,
	) -> Result<bool, VerifierError> {
		unimplemented!()
	}
}

#[cfg(test)]
pub mod test {
	use super::*;
	use celestia_types::blob::GasPrice;
	use m1_da_light_node_util::Config;

	use anyhow::Result;

	/// todo: Investigate why this test sporadically fails.
	#[tokio::test]
	pub async fn test_valid_verifies() -> Result<()> {
		let config = Config::try_from_env()?;
		let client = Arc::new(config.connect_celestia().await?);

		let verifier =
			V1Verifier { client: client.clone(), namespace: config.celestia_namespace.clone() };

		let data = vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
		let blob = Blob::new(config.celestia_namespace.clone(), data.clone())?;

		let height = client.blob_submit(&[blob], GasPrice::default()).await?;

		let included = verifier.verify(VerificationMode::Cowboy, &data, height).await?;

		assert!(included);

		Ok(())
	}

	#[tokio::test]
	pub async fn test_absent_does_not_verify() -> Result<()> {
		let config = Config::try_from_env()?;
		let client = Arc::new(config.connect_celestia().await?);

		let verifier =
			V1Verifier { client: client.clone(), namespace: config.celestia_namespace.clone() };

		let data = vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
		let blob = Blob::new(config.celestia_namespace.clone(), data.clone())?;

		let height = client.blob_submit(&[blob], GasPrice::default()).await?;

		let included = verifier.verify(VerificationMode::Cowboy, &data, height).await?;

		assert!(included);

		let absent_data = vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 7];

		let absent_included = verifier.verify(VerificationMode::Cowboy, &absent_data, height).await;

		match absent_included {
			Ok(_) => {
				assert!(false, "Should not have verified")
			},
			Err(_) => {},
		}

		Ok(())
	}

	#[tokio::test]
	pub async fn test_wrong_height_does_not_verify() -> Result<()> {
		let config = Config::try_from_env()?;
		let client = Arc::new(config.connect_celestia().await?);

		let verifier =
			V1Verifier { client: client.clone(), namespace: config.celestia_namespace.clone() };

		let data = vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
		let blob = Blob::new(config.celestia_namespace.clone(), data.clone())?;

		let height = client.blob_submit(&[blob], GasPrice::default()).await?;

		let included = verifier.verify(VerificationMode::Cowboy, &data, height).await?;

		assert!(included);

		let wrong_height_included =
			verifier.verify(VerificationMode::Cowboy, &data, height + 1).await;

		match wrong_height_included {
			Ok(_) => {
				assert!(false, "Should not have verified")
			},
			Err(_) => {},
		}

		Ok(())
	}
}

