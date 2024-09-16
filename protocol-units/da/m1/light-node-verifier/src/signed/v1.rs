use crate::{Error, Verified, Verifier};
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
	SignatureSize,
};
use m1_da_light_node_util::inner_blob::InnerBlob;
use std::collections::HashSet;

#[derive(Clone)]
pub struct V1Verifier<C>
where
	C: PrimeCurve + CurveArithmetic + DigestPrimitive + PointCompression,
	Scalar<C>: Invert<Output = CtOption<Scalar<C>>> + SignPrimitive<C>,
	SignatureSize<C>: ArrayLength<u8>,
	AffinePoint<C>: FromEncodedPoint<C> + ToEncodedPoint<C> + VerifyPrimitive<C>,
	FieldBytesSize<C>: ModulusSize,
{
	pub _curve_marker: std::marker::PhantomData<C>,
}

#[tonic::async_trait]
impl<C> Verifier<InnerBlob, InnerBlob> for V1Verifier<C>
where
	C: PrimeCurve + CurveArithmetic + DigestPrimitive + PointCompression,
	Scalar<C>: Invert<Output = CtOption<Scalar<C>>> + SignPrimitive<C>,
	SignatureSize<C>: ArrayLength<u8>,
	AffinePoint<C>: FromEncodedPoint<C> + ToEncodedPoint<C> + VerifyPrimitive<C>,
	FieldBytesSize<C>: ModulusSize,
{
	async fn verify(&self, blob: InnerBlob, _height: u64) -> Result<Verified<InnerBlob>, Error> {
		blob.verify_signature::<C>().map_err(|e| Error::Validation(e.to_string()))?;

		Ok(Verified::new(blob))
	}
}

/// Verifies that the signer of the inner blob is in the known signers set.
/// This is built around an inner signer because we should always check the signature first. That is, this composition prevents unsafe usage.
#[derive(Clone)]
pub struct V1InKnownSignersVerifier<C>
where
	C: PrimeCurve + CurveArithmetic + DigestPrimitive + PointCompression,
	Scalar<C>: Invert<Output = CtOption<Scalar<C>>> + SignPrimitive<C>,
	SignatureSize<C>: ArrayLength<u8>,
	AffinePoint<C>: FromEncodedPoint<C> + ToEncodedPoint<C> + VerifyPrimitive<C>,
	FieldBytesSize<C>: ModulusSize,
{
	pub inner_verifier: V1Verifier<C>,
	pub known_signers_sec1_bytes: HashSet<Vec<u8>>,
}

#[tonic::async_trait]
impl<C> Verifier<InnerBlob, InnerBlob> for V1InKnownSignersVerifier<C>
where
	C: PrimeCurve + CurveArithmetic + DigestPrimitive + PointCompression,
	Scalar<C>: Invert<Output = CtOption<Scalar<C>>> + SignPrimitive<C>,
	SignatureSize<C>: ArrayLength<u8>,
	AffinePoint<C>: FromEncodedPoint<C> + ToEncodedPoint<C> + VerifyPrimitive<C>,
	FieldBytesSize<C>: ModulusSize,
{
	async fn verify(&self, blob: InnerBlob, height: u64) -> Result<Verified<InnerBlob>, Error> {
		let inner_blob = self.inner_verifier.verify(blob, height).await?;

		let signer = inner_blob.inner().signer();
		if !self.known_signers_sec1_bytes.contains(signer) {
			return Err(Error::Validation("signer not in known signers".to_string()));
		}

		Ok(inner_blob)
	}
}
