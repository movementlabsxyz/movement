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
	SignatureSize, SigningKey, VerifyingKey,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InnerSignedBlobV1Data {
	pub blob: Vec<u8>,
	pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Id(Vec<u8>);

/// The id for an Ir Blob
impl Id {
	pub fn as_slice(&self) -> &[u8] {
		self.0.as_slice()
	}

	pub fn into_vec(self) -> Vec<u8> {
		self.0
	}
}

impl From<Vec<u8>> for Id {
	fn from(id: Vec<u8>) -> Self {
		Id(id)
	}
}

impl InnerSignedBlobV1Data {
	pub fn new(blob: Vec<u8>, timestamp: u64) -> Self {
		Self { blob, timestamp }
	}

	/// Computes the id of InnerSignedBlobV1Data
	pub fn compute_id<C>(&self) -> Id
	where
		C: PrimeCurve + CurveArithmetic + DigestPrimitive + PointCompression,
		Scalar<C>: Invert<Output = CtOption<Scalar<C>>> + SignPrimitive<C>,
		SignatureSize<C>: ArrayLength<u8>,
		AffinePoint<C>: FromEncodedPoint<C> + ToEncodedPoint<C> + VerifyPrimitive<C>,
		FieldBytesSize<C>: ModulusSize,
	{
		let mut id_hasher = C::Digest::new();
		id_hasher.update(self.blob.as_slice());
		id_hasher.update(&self.timestamp.to_be_bytes());
		Id(id_hasher.finalize().to_vec())
	}

	pub fn try_to_sign<C>(
		self,
		signing_key: &SigningKey<C>,
	) -> Result<InnerSignedBlobV1, anyhow::Error>
	where
		C: PrimeCurve + CurveArithmetic + DigestPrimitive + PointCompression,
		Scalar<C>: Invert<Output = CtOption<Scalar<C>>> + SignPrimitive<C>,
		SignatureSize<C>: ArrayLength<u8>,
		AffinePoint<C>: FromEncodedPoint<C> + ToEncodedPoint<C> + VerifyPrimitive<C>,
		FieldBytesSize<C>: ModulusSize,
	{
		let id = self.compute_id::<C>();
		let mut hasher = C::Digest::new();
		hasher.update(self.blob.as_slice());
		hasher.update(&self.timestamp.to_be_bytes());
		hasher.update(id.as_slice());
		let prehash = hasher.finalize();
		let prehash_bytes = prehash.as_slice();

		let (signature, _recovery_id) = signing_key.sign_prehash_recoverable(prehash_bytes)?;

		Ok(InnerSignedBlobV1 {
			data: self,
			signature: signature.to_vec(),
			signer: signing_key.verifying_key().to_sec1_bytes().to_vec(),
			id,
		})
	}
}

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
pub enum IntermediateBlobRepresentation {
	SignedV1(InnerSignedBlobV1),
}

impl From<InnerSignedBlobV1> for IntermediateBlobRepresentation {
	fn from(inner: InnerSignedBlobV1) -> Self {
		IntermediateBlobRepresentation::SignedV1(inner)
	}
}

impl IntermediateBlobRepresentation {
	pub fn blob(&self) -> &[u8] {
		match self {
			IntermediateBlobRepresentation::SignedV1(inner) => inner.data.blob.as_slice(),
		}
	}

	pub fn signature(&self) -> &[u8] {
		match self {
			IntermediateBlobRepresentation::SignedV1(inner) => inner.signature.as_slice(),
		}
	}

	pub fn timestamp(&self) -> u64 {
		match self {
			IntermediateBlobRepresentation::SignedV1(inner) => inner.data.timestamp,
		}
	}

	pub fn signer(&self) -> &[u8] {
		match self {
			IntermediateBlobRepresentation::SignedV1(inner) => inner.signer.as_slice(),
		}
	}

	pub fn signer_hex(&self) -> String {
		hex::encode(self.signer())
	}

	pub fn id(&self) -> &[u8] {
		match self {
			IntermediateBlobRepresentation::SignedV1(inner) => inner.id.as_slice(),
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
			IntermediateBlobRepresentation::SignedV1(inner) => inner.try_verify::<C>(),
		}
	}
}

#[cfg(test)]
pub mod test {

	use super::*;

	#[test]
	fn test_cannot_change_id_and_verify() -> Result<(), anyhow::Error> {
		let blob = InnerSignedBlobV1Data::new(vec![1, 2, 3], 123);
		let signing_key = SigningKey::<k256::Secp256k1>::random(&mut rand::thread_rng());
		let signed_blob = blob.try_to_sign(&signing_key)?;

		let mut changed_blob = signed_blob.clone();
		changed_blob.id = Id(vec![1, 2, 3, 4]);

		assert!(changed_blob.try_verify::<k256::Secp256k1>().is_err());

		Ok(())
	}
}

pub mod celestia {

	use super::IntermediateBlobRepresentation;
	use anyhow::Context;
	use celestia_types::{consts::appconsts::AppVersion, nmt::Namespace, Blob as CelestiaBlob};

	impl TryFrom<CelestiaBlob> for IntermediateBlobRepresentation {
		type Error = anyhow::Error;

		// todo: it would be nice to have this be self describing over the compression and serialization format
		fn try_from(blob: CelestiaBlob) -> Result<Self, Self::Error> {
			// decompress the blob and deserialize the data with bcs
			let decoder = zstd::Decoder::new(blob.data.as_slice())?;
			let blob = bcs::from_reader(decoder).context("failed to deserialize blob")?;

			Ok(blob)
		}
	}

	pub struct CelestiaIntermediateBlobRepresentation(
		pub IntermediateBlobRepresentation,
		pub Namespace,
	);

	/// Tries to form a CelestiaBlob from a CelestiaIntermediateBlobRepresentation
	impl TryFrom<CelestiaIntermediateBlobRepresentation> for CelestiaBlob {
		type Error = anyhow::Error;

		fn try_from(ir_blob: CelestiaIntermediateBlobRepresentation) -> Result<Self, Self::Error> {
			// Extract the inner blob and namespace
			let CelestiaIntermediateBlobRepresentation(ir_blob, namespace) = ir_blob;

			// Serialize the inner blob with bcs
			let serialized_blob = bcs::to_bytes(&ir_blob).context("failed to serialize blob")?;

			// Compress the serialized data with zstd
			let compressed_blob = zstd::encode_all(serialized_blob.as_slice(), 0)
				.context("failed to compress blob")?;

			// Construct the final CelestiaBlob by assigning the compressed data
			// and associating it with the provided namespace
			Ok(CelestiaBlob::new(namespace, compressed_blob, AppVersion::V2)
				.map_err(|e| anyhow::anyhow!(e))?)
		}
	}
}
