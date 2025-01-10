use crate::cryptography::LocalCryptographySpec;
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
use movement_signer::{cryptography::Curve, SignerError, Signing};

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
	__curve_marker: std::marker::PhantomData<C>,
}

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
	async fn sign(&self, _message: &[u8]) -> Result<C::Signature, SignerError> {
		unimplemented!()
	}

	async fn public_key(&self) -> Result<C::PublicKey, SignerError> {
		unimplemented!()
	}
}
