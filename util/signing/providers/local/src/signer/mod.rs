pub mod ed25519;
pub mod secp256k1;

use crate::cryptography::{LocalCryptographyNoSpec, LocalCryptographySpec};
use ecdsa::signature::Signer as _;
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
	SignatureSize, SigningKey, VerifyingKey,
};
use movement_signer::{
	cryptography::{Curve, TryFromBytes},
	SignerError, Signing,
};

/// [NoSpecLocalSigner] is used to mark a signer tha does not have a cryptography spec, but ideally would.
/// Implementations of [Signing] for [NoSpecLocalSigner] SHALL be bespoke.
pub struct NoSpecLocalSigner<I, C>
where
	C: Curve + LocalCryptographyNoSpec,
{
	inner: I,
	__curve_marker: std::marker::PhantomData<C>,
}

pub struct LocalSigner<C>
where
	C: Curve + LocalCryptographySpec,
	<C as LocalCryptographySpec>::Curve:
		PrimeCurve + CurveArithmetic + DigestPrimitive + PointCompression,
	Scalar<<C as LocalCryptographySpec>::Curve>: Invert<Output = CtOption<Scalar<<C as LocalCryptographySpec>::Curve>>>
		+ SignPrimitive<<C as LocalCryptographySpec>::Curve>,
	SignatureSize<<C as LocalCryptographySpec>::Curve>: ArrayLength<u8>,
	AffinePoint<<C as LocalCryptographySpec>::Curve>: FromEncodedPoint<<C as LocalCryptographySpec>::Curve>
		+ ToEncodedPoint<<C as LocalCryptographySpec>::Curve>
		+ VerifyPrimitive<<C as LocalCryptographySpec>::Curve>,
	FieldBytesSize<<C as LocalCryptographySpec>::Curve>: ModulusSize,
{
	signing_key: SigningKey<<C as LocalCryptographySpec>::Curve>,
	verifying_key: VerifyingKey<<C as LocalCryptographySpec>::Curve>,
	__curve_marker: std::marker::PhantomData<C>,
}

#[async_trait::async_trait]
impl<C> Signing<C> for LocalSigner<C>
where
	C: Curve + LocalCryptographySpec + Send + Sync,
	<C as LocalCryptographySpec>::Curve:
		PrimeCurve + CurveArithmetic + DigestPrimitive + PointCompression,
	Scalar<<C as LocalCryptographySpec>::Curve>: Invert<Output = CtOption<Scalar<<C as LocalCryptographySpec>::Curve>>>
		+ SignPrimitive<<C as LocalCryptographySpec>::Curve>,
	SignatureSize<<C as LocalCryptographySpec>::Curve>: ArrayLength<u8>,
	AffinePoint<<C as LocalCryptographySpec>::Curve>: FromEncodedPoint<<C as LocalCryptographySpec>::Curve>
		+ ToEncodedPoint<<C as LocalCryptographySpec>::Curve>
		+ VerifyPrimitive<<C as LocalCryptographySpec>::Curve>,
	FieldBytesSize<<C as LocalCryptographySpec>::Curve>: ModulusSize,
{
	async fn sign(&self, message: &[u8]) -> Result<C::Signature, SignerError> {
		let (signature, _recovery_id) =
			self.signing_key.try_sign(message).map_err(|e| SignerError::Sign(e.into()))?;
		Ok(C::Signature::try_from_bytes(signature.to_vec().as_slice())
			.map_err(|e| SignerError::Sign(e.into()))?)
	}

	async fn public_key(&self) -> Result<C::PublicKey, SignerError> {
		C::PublicKey::try_from_bytes(self.verifying_key.to_encoded_point(false).as_ref())
			.map_err(|e| SignerError::PublicKey(e.into()))
	}
}
