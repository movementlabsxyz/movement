use crate::blob::ir::data::InnerSignedBlobV1Data;
use crate::blob::ir::id::Id;
use ecdsa::{
	elliptic_curve::{
		generic_array::ArrayLength,
		ops::Invert,
		point::PointCompression,
		sec1::{FromEncodedPoint, ModulusSize, ToEncodedPoint},
		subtle::CtOption,
		AffinePoint, CurveArithmetic, FieldBytesSize, PrimeCurve, Scalar,
	},
	hazmat::{DigestPrimitive, SignPrimitive, VerifyPrimitive},
	signature::{digest::Digest, DigestVerifier},
	SignatureSize, VerifyingKey,
};
use movement_da_light_node_proto::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InnerSignedBlobV1 {
	pub data: InnerSignedBlobV1Data,
	pub signature: Vec<u8>,
	pub signer: Vec<u8>,
	pub id: Id,
}

impl InnerSignedBlobV1 {
	pub fn try_verify<C>(&self) -> Result<(), anyhow::Error>
	where
		C: PrimeCurve + CurveArithmetic + DigestPrimitive + PointCompression,
		Scalar<C>: Invert<Output = CtOption<Scalar<C>>> + SignPrimitive<C>,
		SignatureSize<C>: ArrayLength<u8>,
		AffinePoint<C>: FromEncodedPoint<C> + ToEncodedPoint<C> + VerifyPrimitive<C>,
		FieldBytesSize<C>: ModulusSize,
	{
		let mut hasher = C::Digest::new();
		hasher.update(self.data.blob.as_slice());
		hasher.update(&self.data.timestamp.to_be_bytes());
		hasher.update(self.id.as_slice());

		let verifying_key = VerifyingKey::<C>::from_sec1_bytes(self.signer.as_slice())?;
		let signature = ecdsa::Signature::from_bytes(self.signature.as_slice().into())?;

		match verifying_key.verify_digest(hasher, &signature) {
			Ok(_) => Ok(()),
			Err(_) => Err(anyhow::anyhow!("Failed to verify signature")),
		}
	}
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DaBlob {
	SignedV1(InnerSignedBlobV1),
	DigestV1(Vec<u8>),
}

impl From<InnerSignedBlobV1> for DaBlob {
	fn from(inner: InnerSignedBlobV1) -> Self {
		DaBlob::SignedV1(inner)
	}
}

impl DaBlob {
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

	pub fn verify_signature<C>(&self) -> Result<(), anyhow::Error>
	where
		C: PrimeCurve + CurveArithmetic + DigestPrimitive + PointCompression,
		Scalar<C>: Invert<Output = CtOption<Scalar<C>>> + SignPrimitive<C>,
		SignatureSize<C>: ArrayLength<u8>,
		AffinePoint<C>: FromEncodedPoint<C> + ToEncodedPoint<C> + VerifyPrimitive<C>,
		FieldBytesSize<C>: ModulusSize,
	{
		match self {
			DaBlob::SignedV1(inner) => inner.try_verify::<C>(),
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
	pub fn to_blob_heartbeat_response(self) -> BlobResponse {
		//for heartbeat blob the data are removed.
		let blob = Blob {
			data: vec![],
			signature: self.signature().to_vec(),
			timestamp: self.timestamp(),
			signer: self.signer().to_vec(),
			blob_id: self.id().to_vec(),
			height: 0,
		};
		BlobResponse { blob_type: Some(blob_response::BlobType::HeartbeatBlob(blob)) }
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
	use ecdsa::SigningKey;

	#[test]
	fn test_cannot_change_id_and_verify() -> Result<(), anyhow::Error> {
		let blob = InnerSignedBlobV1Data::new(vec![1, 2, 3], 123);
		let signing_key = SigningKey::<k256::Secp256k1>::random(&mut rand::thread_rng());
		let signed_blob = blob.try_to_sign(&signing_key)?;

		let mut changed_blob = signed_blob.clone();
		changed_blob.id = Id::new(vec![1, 2, 3, 4]);

		assert!(changed_blob.try_verify::<k256::Secp256k1>().is_err());

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
