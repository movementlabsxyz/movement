use crate::blob::ir::blob::InnerSignedBlobV1;
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
	signature::digest::Digest,
	SignatureSize, SigningKey,
};
use serde::{Deserialize, Serialize};

/// The data that should be signed before submission.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InnerSignedBlobV1Data {
	pub blob: Vec<u8>,
	pub timestamp: u64,
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
		Id::new(id_hasher.finalize().to_vec())
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
