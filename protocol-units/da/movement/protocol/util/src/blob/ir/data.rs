use crate::blob::ir::blob::InnerSignedBlobV1;
use crate::blob::ir::id::Id;
use movement_da_light_node_signer::Signer;
use movement_signer::{
	cryptography::{Curve, ToBytes},
	Digester, Signing, Verify,
};
use serde::{Deserialize, Serialize};
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InnerSignedBlobV1Data<C>
where
	C: Curve,
{
	pub blob: Vec<u8>,
	pub timestamp: u64,
	#[serde(skip)]
	__curve_marker: std::marker::PhantomData<C>,
}

impl<C> InnerSignedBlobV1Data<C>
where
	C: Curve + Verify<C> + Digester<C>,
{
	pub fn new(blob: Vec<u8>, timestamp: u64) -> Self {
		Self { blob, timestamp, __curve_marker: std::marker::PhantomData }
	}

	pub fn now(blob: Vec<u8>) -> Self {
		// mark the timestamp as now in milliseconds
		let timestamp = chrono::Utc::now().timestamp_micros() as u64;

		Self::new(blob, timestamp)
	}

	/// Gets an owned copy of the bytes to be signed
	fn to_signing_bytes(&self) -> Vec<u8> {
		[self.blob.as_slice(), &self.timestamp.to_be_bytes()].concat()
	}

	/// Computes the id of InnerSignedBlobV1Data
	pub fn compute_id(&self) -> Result<Id, anyhow::Error> {
		let byte_slice = self.to_signing_bytes();

		Ok(Id::new(C::digest(&byte_slice)?.to_bytes()))
	}

	pub async fn try_to_sign<O>(
		self,
		signer: &Signer<O, C>,
	) -> Result<InnerSignedBlobV1<C>, anyhow::Error>
	where
		O: Signing<C>,
		C: Curve + Digester<C>,
	{
		let id = self.compute_id()?;
		info!("Signing blob with id {:?}", id);
		let signature = signer.inner().sign(&id.as_slice()).await?.to_bytes();
		let signer = signer.inner().public_key().await?.to_bytes();

		Ok(InnerSignedBlobV1::new(self, signature, signer, id))
	}
}

pub mod block {

	use super::*;
	use movement_types::block;

	impl<C> TryFrom<block::Block> for InnerSignedBlobV1Data<C>
	where
		C: Curve + Verify<C> + Digester<C>,
	{
		type Error = anyhow::Error;

		fn try_from(block: block::Block) -> Result<Self, Self::Error> {
			let blob = bcs::to_bytes(&block)?;
			Ok(Self::now(blob))
		}
	}

	impl<C> TryFrom<block::Id> for InnerSignedBlobV1Data<C>
	where
		C: Curve + Verify<C> + Digester<C>,
	{
		type Error = anyhow::Error;

		fn try_from(id: block::Id) -> Result<Self, Self::Error> {
			let blob = id.as_bytes().to_vec();
			Ok(Self::now(blob))
		}
	}

	impl<C> TryFrom<Vec<block::Id>> for InnerSignedBlobV1Data<C>
	where
		C: Curve + Verify<C> + Digester<C>,
	{
		type Error = anyhow::Error;

		fn try_from(ids: Vec<block::Id>) -> Result<Self, Self::Error> {
			let blob = bcs::to_bytes(&ids)?;
			Ok(Self::now(blob))
		}
	}
}
