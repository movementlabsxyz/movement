use movement_da_light_node_signer::Signer;
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

pub mod block {

	use super::*;
	use movement_types::block;

	impl TryFrom<block::Block> for InnerSignedBlobV1Data {
		type Error = anyhow::Error;

		fn try_from(block: block::Block) -> Result<Self, Self::Error> {
			let blob = bcs::to_bytes(&block)?;
			Ok(Self::now(blob))
		}
	}

	impl TryFrom<block::Id> for InnerSignedBlobV1Data {
		type Error = anyhow::Error;

		fn try_from(id: block::Id) -> Result<Self, Self::Error> {
			let blob = id.as_bytes().to_vec();
			Ok(Self::now(blob))
		}
	}

	impl TryFrom<Vec<block::Id>> for InnerSignedBlobV1Data {
		type Error = anyhow::Error;

		fn try_from(ids: Vec<block::Id>) -> Result<Self, Self::Error> {
			let blob = bcs::to_bytes(&ids)?;
			Ok(Self::now(blob))
		}
	}
}
