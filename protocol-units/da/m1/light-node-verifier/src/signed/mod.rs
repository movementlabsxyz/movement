use crate::{Error, Verified, VerifierOperations};
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
use m1_da_light_node_util::ir_blob::IntermediateBlobRepresentation;
use std::collections::HashSet;
use tracing::info;

/// A verifier that checks the signature of the inner blob.
#[derive(Clone)]
pub struct Verifier<C>
where
	C: PrimeCurve + CurveArithmetic + DigestPrimitive + PointCompression,
	Scalar<C>: Invert<Output = CtOption<Scalar<C>>> + SignPrimitive<C>,
	SignatureSize<C>: ArrayLength<u8>,
	AffinePoint<C>: FromEncodedPoint<C> + ToEncodedPoint<C> + VerifyPrimitive<C>,
	FieldBytesSize<C>: ModulusSize,
{
	pub _curve_marker: std::marker::PhantomData<C>,
}

impl<C> Verifier<C>
where
	C: PrimeCurve + CurveArithmetic + DigestPrimitive + PointCompression,
	Scalar<C>: Invert<Output = CtOption<Scalar<C>>> + SignPrimitive<C>,
	SignatureSize<C>: ArrayLength<u8>,
	AffinePoint<C>: FromEncodedPoint<C> + ToEncodedPoint<C> + VerifyPrimitive<C>,
	FieldBytesSize<C>: ModulusSize,
{
	pub fn new() -> Self {
		Self { _curve_marker: std::marker::PhantomData }
	}
}

#[tonic::async_trait]
impl<C> VerifierOperations<IntermediateBlobRepresentation, IntermediateBlobRepresentation>
	for Verifier<C>
where
	C: PrimeCurve + CurveArithmetic + DigestPrimitive + PointCompression,
	Scalar<C>: Invert<Output = CtOption<Scalar<C>>> + SignPrimitive<C>,
	SignatureSize<C>: ArrayLength<u8>,
	AffinePoint<C>: FromEncodedPoint<C> + ToEncodedPoint<C> + VerifyPrimitive<C>,
	FieldBytesSize<C>: ModulusSize,
{
	async fn verify(
		&self,
		blob: IntermediateBlobRepresentation,
		_height: u64,
	) -> Result<Verified<IntermediateBlobRepresentation>, Error> {
		blob.verify_signature::<C>().map_err(|e| Error::Validation(e.to_string()))?;

		Ok(Verified::new(blob))
	}
}

/// Verifies that the signer of the inner blob is in the known signers set.
/// This is built around an inner signer because we should always check the signature first. That is, this composition prevents unsafe usage.
#[derive(Clone)]
pub struct InKnownSignersVerifier<C>
where
	C: PrimeCurve + CurveArithmetic + DigestPrimitive + PointCompression,
	Scalar<C>: Invert<Output = CtOption<Scalar<C>>> + SignPrimitive<C>,
	SignatureSize<C>: ArrayLength<u8>,
	AffinePoint<C>: FromEncodedPoint<C> + ToEncodedPoint<C> + VerifyPrimitive<C>,
	FieldBytesSize<C>: ModulusSize,
{
	pub inner_verifier: Verifier<C>,
	/// The set of known signers in sec1 bytes hex format.
	pub known_signers_sec1_bytes_hex: HashSet<String>,
}

impl<C> InKnownSignersVerifier<C>
where
	C: PrimeCurve + CurveArithmetic + DigestPrimitive + PointCompression,
	Scalar<C>: Invert<Output = CtOption<Scalar<C>>> + SignPrimitive<C>,
	SignatureSize<C>: ArrayLength<u8>,
	AffinePoint<C>: FromEncodedPoint<C> + ToEncodedPoint<C> + VerifyPrimitive<C>,
	FieldBytesSize<C>: ModulusSize,
{
	pub fn new<T>(known_signers_sec1_bytes_hex: T) -> Self
	where
		T: IntoIterator,
		T::Item: Into<String>,
	{
		Self {
			inner_verifier: Verifier::new(),
			known_signers_sec1_bytes_hex: known_signers_sec1_bytes_hex
				.into_iter()
				.map(Into::into)
				.collect(),
		}
	}
}

#[tonic::async_trait]
impl<C> VerifierOperations<IntermediateBlobRepresentation, IntermediateBlobRepresentation>
	for InKnownSignersVerifier<C>
where
	C: PrimeCurve + CurveArithmetic + DigestPrimitive + PointCompression,
	Scalar<C>: Invert<Output = CtOption<Scalar<C>>> + SignPrimitive<C>,
	SignatureSize<C>: ArrayLength<u8>,
	AffinePoint<C>: FromEncodedPoint<C> + ToEncodedPoint<C> + VerifyPrimitive<C>,
	FieldBytesSize<C>: ModulusSize,
{
	async fn verify(
		&self,
		blob: IntermediateBlobRepresentation,
		height: u64,
	) -> Result<Verified<IntermediateBlobRepresentation>, Error> {
		let ir_blob = self.inner_verifier.verify(blob, height).await?;
		info!("Verified inner blob");
		let signer = ir_blob.inner().signer_hex();
		if !self.known_signers_sec1_bytes_hex.contains(&signer) {
			return Err(Error::Validation("signer not in known signers".to_string()));
		}

		Ok(ir_blob)
	}
}

#[cfg(test)]
pub mod tests {
	// TODO: we need to recreate the signed verifier tests
}
