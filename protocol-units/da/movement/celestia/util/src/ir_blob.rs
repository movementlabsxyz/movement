use movement_celestia_light_node_signer::Signer;
use movement_signer::{
	cryptography::{Curve, ToBytes, TryFromBytes},
	Digester, Signing, Verify,
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

	/// Gets an owned copy of the bytes to be signed
	fn to_signing_bytes(&self) -> Vec<u8> {
		[self.blob.as_slice(), &self.timestamp.to_be_bytes()].concat()
	}

	/// Computes the id of InnerSignedBlobV1Data
	pub fn compute_id<O, C>(&self) -> Result<Id, anyhow::Error>
	where
		C: Curve + Digester<C>,
	{
		let byte_slice = self.to_signing_bytes();

		Ok(Id(C::digest(&byte_slice)?.to_bytes()))
	}

	pub async fn try_to_sign<O, C>(
		self,
		signer: &Signer<O, C>,
	) -> Result<InnerSignedBlobV1, anyhow::Error>
	where
		O: Signing<C>,
		C: Curve + Digester<C>,
	{
		let id = self.compute_id::<O, C>()?;
		let signature = signer.inner().sign(&id.as_slice()).await?.to_bytes();
		let signer = signer.inner().public_key().await?.to_bytes();

		Ok(InnerSignedBlobV1 { data: self, signature, signer, id })
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
		C: Curve + Verify<C>,
	{
		let public_key = C::PublicKey::try_from_bytes(self.signer.as_slice())?;
		let signature = C::Signature::try_from_bytes(self.signature.as_slice())?;

		if !C::verify(self.data.blob.as_slice(), &signature, &public_key)? {
			return Err(anyhow::anyhow!("signature verification failed"))?;
		}

		Ok(())
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
		C: Curve + Verify<C>,
	{
		match self {
			IntermediateBlobRepresentation::SignedV1(inner) => inner.try_verify::<C>(),
		}
	}
}

#[cfg(test)]
pub mod test {

	use super::*;
	use movement_signer::cryptography::secp256k1::Secp256k1;
	use movement_signer_local::signer::LocalSigner;

	#[tokio::test]
	async fn test_cannot_change_id_and_verify() -> Result<(), anyhow::Error> {
		let blob = InnerSignedBlobV1Data::new(vec![1, 2, 3], 123);
		let signer = Signer::new(LocalSigner::<Secp256k1>::random());
		let signed_blob = blob.try_to_sign(&signer).await?;

		let mut changed_blob = signed_blob.clone();
		changed_blob.id = Id(vec![1, 2, 3, 4]);

		assert!(changed_blob.try_verify::<Secp256k1>().is_err());

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
			// decompress blob.data with zstd
			let decompressed =
				zstd::decode_all(blob.data.as_slice()).context("failed to decompress blob")?;

			// deserialize the decompressed data with bcs
			let blob =
				bcs::from_bytes(decompressed.as_slice()).context("failed to deserialize blob")?;

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
