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
	SignatureSize, SigningKey, VerifyingKey,
};
use movement_signer::{
	cryptography::{secp256k1::Secp256k1, Curve, TryFromBytes},
	SignerError, Signing,
};

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

impl LocalSigner<Secp256k1> {
	/// Constructs a new [LocalSigner] with the provided key pair.
	pub fn new(
		signing_key: SigningKey<k256::Secp256k1>,
		verifying_key: VerifyingKey<k256::Secp256k1>,
	) -> Self {
		Self { signing_key, verifying_key, __curve_marker: std::marker::PhantomData }
	}

	/// Constructs a new [LocalSigner] with a random key pair.
	pub fn random() -> Self {
		let signing_key = SigningKey::<k256::Secp256k1>::random(&mut rand::thread_rng());

		let verifying_key = signing_key.verifying_key().clone();
		Self::new(signing_key, verifying_key)
	}

	/// Constructs a new [LocalSigner] from a [SigningKey].
	pub fn from_signing_key(signing_key: SigningKey<k256::Secp256k1>) -> Self {
		let verifying_key = signing_key.verifying_key().clone();
		Self::new(signing_key, verifying_key)
	}

	/// Constructs a new [LocalSigner] from a byte slice.
	pub fn from_signing_key_bytes(bytes: &[u8]) -> Result<Self, SignerError> {
		let signing_key_bytes: &[u8; 32] =
			bytes.try_into().map_err(|_| SignerError::Decode("Invalid key length".into()))?;
		let signing_key = SigningKey::<k256::Secp256k1>::from_bytes(signing_key_bytes.into())
			.map_err(|e| SignerError::Decode(e.into()))?;
		Ok(Self::from_signing_key(signing_key))
	}

	pub fn from_signing_key_hex(hex: &str) -> Result<Self, SignerError> {
		let bytes = hex::decode(hex).map_err(|e| SignerError::Decode(anyhow::anyhow!(e).into()))?;
		Self::from_signing_key_bytes(&bytes)
	}
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
		let (signature, _recovery_id) = self
			.signing_key
			.sign_prehash_recoverable(message)
			.map_err(|e| SignerError::Sign(e.into()))?;
		Ok(C::Signature::try_from_bytes(signature.to_vec().as_slice())
			.map_err(|e| SignerError::Sign(e.into()))?)
	}

	async fn public_key(&self) -> Result<C::PublicKey, SignerError> {
		C::PublicKey::try_from_bytes(self.verifying_key.to_encoded_point(false).as_bytes())
			.map_err(|e| SignerError::PublicKey(e.into()))
	}
}
