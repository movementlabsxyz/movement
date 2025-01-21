use crate::blob::ir::data::InnerSignedBlobV1Data;
use crate::blob::ir::id::Id;
use movement_da_light_node_proto::*;
use movement_signer::{
	cryptography::{Curve, TryFromBytes},
	Digester, Verify,
};
use serde::{Deserialize, Serialize};
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InnerSignedBlobV1<C>
where
	C: Curve,
{
	pub data: InnerSignedBlobV1Data<C>,
	pub signature: Vec<u8>,
	pub signer: Vec<u8>,
	pub id: Id,
}

impl<C> InnerSignedBlobV1<C>
where
	C: Curve + Verify<C> + Digester<C>,
{
	pub fn new(
		data: InnerSignedBlobV1Data<C>,
		signature: Vec<u8>,
		signer: Vec<u8>,
		id: Id,
	) -> Self {
		Self { data, signature, signer, id }
	}

	pub fn try_verify(&self) -> Result<(), anyhow::Error> {
		let public_key = C::PublicKey::try_from_bytes(self.signer.as_slice())?;
		let signature = C::Signature::try_from_bytes(self.signature.as_slice())?;
		let message = self.data.compute_id()?;
		info!("verifying signature for message {:?}", message);

		if !C::verify(message.as_slice(), &signature, &public_key)? {
			return Err(anyhow::anyhow!("signature verification failed"))?;
		}

		Ok(())
	}
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DaBlob<C>
where
	C: Curve,
{
	SignedV1(InnerSignedBlobV1<C>),
	DigestV1(Vec<u8>),
}

impl<C> From<InnerSignedBlobV1<C>> for DaBlob<C>
where
	C: Curve,
{
	fn from(inner: InnerSignedBlobV1<C>) -> Self {
		DaBlob::SignedV1(inner)
	}
}

impl<C> DaBlob<C>
where
	C: Curve + Verify<C> + Digester<C>,
{
	pub fn blob(&self) -> &[u8] {
		match self {
			DaBlob::SignedV1(inner) => inner.data.blob.as_slice(),
			DaBlob::DigestV1(digest) => digest.as_slice(),
		}
	}

	pub fn signature(&self) -> &[u8] {
		match self {
			DaBlob::SignedV1(inner) => inner.signature.as_slice(),
			DaBlob::DigestV1(_) => &[],
		}
	}

	pub fn timestamp(&self) -> u64 {
		match self {
			DaBlob::SignedV1(inner) => inner.data.timestamp,
			DaBlob::DigestV1(_) => 0,
		}
	}

	pub fn signer(&self) -> &[u8] {
		match self {
			DaBlob::SignedV1(inner) => inner.signer.as_slice(),
			DaBlob::DigestV1(_) => &[],
		}
	}

	pub fn signer_hex(&self) -> String {
		hex::encode(self.signer())
	}

	pub fn id(&self) -> &[u8] {
		match self {
			DaBlob::SignedV1(inner) => inner.id.as_slice(),
			DaBlob::DigestV1(digest) => digest.as_slice(),
		}
	}

	pub fn verify_signature(&self) -> Result<(), anyhow::Error> {
		match self {
			DaBlob::SignedV1(inner) => inner.try_verify(),
			DaBlob::DigestV1(_) => Ok(()),
		}
	}

	pub fn to_blob(self, height: u64) -> Result<Blob, anyhow::Error> {
		Ok(Blob {
			data: self.blob().to_vec(),
			signature: self.signature().to_vec(),
			timestamp: self.timestamp(),
			signer: self.signer().to_vec(),
			blob_id: self.id().to_vec(),
			height,
		})
	}

	pub fn blob_to_blob_write_response(blob: Blob) -> Result<BlobResponse, anyhow::Error> {
		Ok(BlobResponse { blob_type: Some(blob_response::BlobType::PassedThroughBlob(blob)) })
	}

	/// Converts a [Blob] into a [BlobResponse] with the blob passed through.
	pub fn blob_to_blob_passed_through_read_response(
		blob: Blob,
	) -> Result<BlobResponse, anyhow::Error> {
		Ok(BlobResponse { blob_type: Some(blob_response::BlobType::PassedThroughBlob(blob)) })
	}

	/// Converts a [Blob] into a [BlobResponse] with the blob sequenced.
	pub fn blob_to_blob_sequenced_read_response(blob: Blob) -> Result<BlobResponse, anyhow::Error> {
		Ok(BlobResponse { blob_type: Some(blob_response::BlobType::SequencedBlobBlock(blob)) })
	}

	/// Converts a [DaBlob] into a [BlobResponse] with the blob passed through.
	pub fn to_blob_passed_through_read_response(
		self,
		height: u64,
	) -> Result<BlobResponse, anyhow::Error> {
		let blob = self.to_blob(height)?;
		Self::blob_to_blob_passed_through_read_response(blob)
	}

	/// Converts a [DaBlob] into a [BlobResponse] with the blob sequenced.
	pub fn to_blob_sequenced_read_response(
		self,
		height: u64,
	) -> Result<BlobResponse, anyhow::Error> {
		let blob = self.to_blob(height)?;
		Self::blob_to_blob_sequenced_read_response(blob)
	}
}

#[cfg(test)]
pub mod test {

	use super::*;
	use movement_da_light_node_signer::Signer;
	use movement_signer::cryptography::secp256k1::Secp256k1;
	use movement_signer_local::signer::LocalSigner;

	#[tokio::test]
	async fn test_cannot_change_id_and_verify() -> Result<(), anyhow::Error> {
		let blob = InnerSignedBlobV1Data::new(vec![1, 2, 3], 123);
		let signer = Signer::new(LocalSigner::<Secp256k1>::random());
		let signed_blob = blob.try_to_sign(&signer).await?;

		let mut changed_blob = signed_blob.clone();
		changed_blob.id = Id::new(vec![1, 2, 3, 4]);

		assert!(changed_blob.try_verify().is_err());

		Ok(())
	}
}

pub mod stream_read_response {

	use movement_da_light_node_proto::*;

	/// Converts a passed through [BlobResponse] into a sequenced [BlobResponse].
	pub fn passed_through_to_sequenced(blob_response: BlobResponse) -> BlobResponse {
		match blob_response.blob_type {
			Some(blob_response::BlobType::PassedThroughBlob(blob)) => {
				BlobResponse { blob_type: Some(blob_response::BlobType::SequencedBlobBlock(blob)) }
			}
			_ => blob_response,
		}
	}

	/// Converts a passed through [StreamReadFromHeightResponse] into a sequenced [StreamReadFromHeightResponse].
	pub fn passed_through_to_sequenced_response(
		response: StreamReadFromHeightResponse,
	) -> StreamReadFromHeightResponse {
		StreamReadFromHeightResponse { blob: response.blob.map(passed_through_to_sequenced) }
	}
}
