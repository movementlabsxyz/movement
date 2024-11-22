use crate::{Error, Verified, VerifierOperations};
use celestia_rpc::{BlobClient, Client, HeaderClient};
use celestia_types::{consts::appconsts::AppVersion, nmt::Namespace, Blob};
use m1_da_light_node_util::ir_blob::IntermediateBlobRepresentation;
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
		height: u64,
	) -> Result<Verified<IntermediateBlobRepresentation>, Error> {
		//@l-monninger: the light node itself does most of the work of verify blobs. The verification under the feature flag below is useful in zero-trust environments.

		blob.validate(AppVersion::V2).map_err(|e| Error::Validation(e.to_string()))?;

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

		let ir_blob = IntermediateBlobRepresentation::try_from(blob)
			.map_err(|e| Error::Internal(e.to_string()))?;

		Ok(Verified::new(ir_blob))
	}
}
