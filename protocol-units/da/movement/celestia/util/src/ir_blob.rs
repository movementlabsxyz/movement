use anyhow::bail;
use ecdsa::{
	elliptic_curve::{
		generic_array::typenum::Unsigned,
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

/// Maximum allowed length of a blob data field.
pub const MAX_BLOB_LEN: usize = 64 * 1_024 * 1_024;

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
	pub fn try_new(blob: Vec<u8>, timestamp: u64) -> Result<Self, anyhow::Error> {
		if blob.len() > MAX_BLOB_LEN {
			bail!("blob length {} is above the limit", blob.len());
		}
		Ok(Self { blob, timestamp })
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
		if self.signature.len() != SignatureSize::<C>::to_usize() {
			return Err(anyhow::anyhow!("invalid signature length"));
		}
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
		let blob = InnerSignedBlobV1Data::try_new(vec![1, 2, 3], 123).unwrap();
		let signing_key = SigningKey::<k256::Secp256k1>::random(&mut rand::thread_rng());
		let signed_blob = blob.try_to_sign(&signing_key)?;

		let mut changed_blob = signed_blob.clone();
		changed_blob.id = Id(vec![1, 2, 3, 4]);

		assert!(changed_blob.try_verify::<k256::Secp256k1>().is_err());

		Ok(())
	}

	#[test]
	fn poc_verify_does_not_panic_on_wrong_signature_len() -> Result<(), anyhow::Error> {
		let s = InnerSignedBlobV1 {
			data: InnerSignedBlobV1Data::try_new(vec![1, 2, 3], 123).unwrap(),
			signature: vec![],
			signer: vec![
				2, 130, 130, 130, 130, 130, 130, 130, 82, 130, 130, 130, 130, 255, 255, 130, 130,
				130, 130, 130, 130, 130, 130, 130, 130, 130, 130, 130, 130, 130, 130, 130, 130,
			],
			id: Id(vec![1, 2, 3, 4]),
		};

		s.try_verify::<k256::Secp256k1>().unwrap_err();

		Ok(())
	}
}

pub mod celestia {
	use super::{IntermediateBlobRepresentation, MAX_BLOB_LEN};
	use anyhow::Context;
	use celestia_types::{consts::appconsts::AppVersion, nmt::Namespace, Blob as CelestiaBlob};

	impl TryFrom<CelestiaBlob> for IntermediateBlobRepresentation {
		type Error = anyhow::Error;

		// todo: it would be nice to have this be self describing over the compression and serialization format
		fn try_from(blob: CelestiaBlob) -> Result<Self, Self::Error> {
			// decompress the blob and deserialize the data with bcs
			let mut decoder = zstd::Decoder::with_buffer(blob.data.as_slice())?;
			let blob = bcs::de::Builder::new()
				.max_sequence_length(MAX_BLOB_LEN)
				.deserialize_reader(&mut decoder)
				.context("failed to deserialize blob")?;

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

			let mut encoder =
				zstd::Encoder::new(vec![], 0).context("failed to initialize zstd encoder")?;

			// Serialize the inner blob with bcs and compress with zstd
			bcs::serialize_into(&mut encoder, &ir_blob).context("failed to serialize blob")?;

			let compressed_blob =
				encoder.finish().context("failed to finish compression of blob")?;

			// Construct the final CelestiaBlob by assigning the compressed data
			// and associating it with the provided namespace
			Ok(CelestiaBlob::new(namespace, compressed_blob, AppVersion::V2)
				.map_err(|e| anyhow::anyhow!(e))?)
		}
	}

	#[cfg(test)]
	mod tests {
		use super::*;
		use crate::ir_blob::{InnerSignedBlobV1, InnerSignedBlobV1Data};

		#[test]
		fn zstd_bomb() -> anyhow::Result<()> {
			// MAGIC + header with max window size
			let mut bomb = vec![0x28, 0xb5, 0x2f, 0xfd, 0x0, 0x7f];
			let n_blocks = 0x530000;
			for _ in 0..n_blocks {
				// RLE block encoding 0xff byte repeated 0x8000 times
				bomb.extend([0x02, 0x00, 0x10, 0xff]);
			}
			// Block to finish the data
			bomb.extend(&[0x01, 0x00, 0x00]);
			// Check that we fit in celestia limits
			assert!(bomb.len() < 0x1_500_000);

			let blob =
				CelestiaBlob::new(Namespace::new_v0(b"movement").unwrap(), bomb, AppVersion::V2)?;
			<CelestiaBlob as TryInto<IntermediateBlobRepresentation>>::try_into(blob).unwrap_err();
			Ok(())
		}

		fn dummy_ir_blob(len: usize) -> CelestiaIntermediateBlobRepresentation {
			let blob_data = InnerSignedBlobV1Data { blob: vec![0; len], timestamp: 1733879282 };
			// It's no fun to compute -- not Kraftwerk
			let test_blob = InnerSignedBlobV1 {
				data: blob_data,
				signature: vec![0xfa; 64],
				signer: vec![0xaf; 32],
				id: vec![0xad; 32].into(),
			};
			CelestiaIntermediateBlobRepresentation(
				test_blob.into(),
				Namespace::new_v0(b"movement").unwrap(),
			)
		}

		#[test]
		#[ignore = "allocates, compresses, and decompresses 2 GiB of data"]
		fn blob_size_limit_imposed_by_bcs() -> anyhow::Result<()> {
			CelestiaBlob::try_from(dummy_ir_blob(bcs::MAX_SEQUENCE_LENGTH + 1))
				.expect_err("should be rejected");

			let celestia_blob: CelestiaBlob = dummy_ir_blob(bcs::MAX_SEQUENCE_LENGTH).try_into()?;
			let blob_ir: IntermediateBlobRepresentation = celestia_blob.try_into()?;
			assert_eq!(blob_ir.blob().len(), bcs::MAX_SEQUENCE_LENGTH);
			Ok(())
		}
	}
}
