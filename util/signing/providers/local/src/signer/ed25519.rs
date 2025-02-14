use crate::signer::NoSpecLocalSigner;
use ed25519_dalek::{Signer as _, SigningKey, VerifyingKey};
use movement_signer::Signing;
use movement_signer::{
	cryptography::{ed25519::Ed25519, Curve, TryFromBytes},
	SignerError,
};

pub struct Ed25519SignerInner {
	signing_key: SigningKey,
	verifying_key: VerifyingKey,
}

impl Ed25519SignerInner {
	/// Constructs a new [NoSpecLocalSigner] with the provided key pair.
	pub fn new(signing_key: SigningKey, verifying_key: VerifyingKey) -> Self {
		Self { signing_key, verifying_key }
	}

	/// Constructs a new [NoSpecLocalSigner] with a random key pair.
	pub fn random() -> Self {
		let signing_key = SigningKey::generate(&mut rand::thread_rng());

		let verifying_key = signing_key.verifying_key().clone();
		Self::new(signing_key, verifying_key)
	}

	/// Constructs a new [NoSpecLocalSigner] from a [SigningKey].
	pub fn from_signing_key(signing_key: SigningKey) -> Self {
		let verifying_key = signing_key.verifying_key().clone();
		Self::new(signing_key, verifying_key)
	}

	/// Constructs a new [NoSpecLocalSigner] from a byte slice.
	pub fn from_signing_key_bytes(bytes: &[u8]) -> Result<Self, SignerError> {
		let signing_key_bytes: &[u8; 32] =
			bytes.try_into().map_err(|_| SignerError::Decode("Invalid key length".into()))?;
		let signing_key = SigningKey::from_bytes(signing_key_bytes.into());
		Ok(Self::from_signing_key(signing_key))
	}

	/// Constructs a new [NoSpecLocalSigner] from a hex string.
	pub fn from_signing_key_hex(hex: &str) -> Result<Self, SignerError> {
		let bytes = hex::decode(hex).map_err(|e| {
			SignerError::Decode(format!("failed to decode hex string: {}", e).into())
		})?;
		Self::from_signing_key_bytes(&bytes)
	}
}

impl NoSpecLocalSigner<Ed25519SignerInner, Ed25519> {
	/// Constructs a new [NoSpecLocalSigner] with the provided key pair.
	pub fn new(signing_key: SigningKey, verifying_key: VerifyingKey) -> Self {
		Self {
			inner: Ed25519SignerInner::new(signing_key, verifying_key),
			__curve_marker: std::marker::PhantomData,
		}
	}

	/// Constructs a new [NoSpecLocalSigner] with a random key pair.
	pub fn random() -> Self {
		let inner = Ed25519SignerInner::random();

		Self { inner, __curve_marker: std::marker::PhantomData }
	}

	/// Constructs a new [NoSpecLocalSigner] from a [SigningKey].
	pub fn from_signing_key(signing_key: SigningKey) -> Self {
		let verifying_key = signing_key.verifying_key().clone();
		Self::new(signing_key, verifying_key)
	}

	/// Constructs a new [NoSpecLocalSigner] from a byte slice.
	pub fn from_signing_key_bytes(bytes: &[u8]) -> Result<Self, SignerError> {
		let inner = Ed25519SignerInner::from_signing_key_bytes(bytes)?;
		Ok(Self { inner, __curve_marker: std::marker::PhantomData })
	}

	/// Constructs a new [NoSpecLocalSigner] from a hex string.
	pub fn from_signing_key_hex(hex: &str) -> Result<Self, SignerError> {
		let bytes = hex::decode(hex).map_err(|e| {
			SignerError::Decode(format!("failed to decode hex string: {}", e).into())
		})?;
		Self::from_signing_key_bytes(&bytes)
	}
}

#[async_trait::async_trait]
impl Signing<Ed25519> for NoSpecLocalSigner<Ed25519SignerInner, Ed25519> {
	async fn sign(&self, message: &[u8]) -> Result<<Ed25519 as Curve>::Signature, SignerError> {
		let signature = self
			.inner
			.signing_key
			.try_sign(message)
			.map_err(|e| SignerError::Sign(e.into()))?;
		Ok(<Ed25519 as Curve>::Signature::try_from_bytes(signature.to_vec().as_slice())
			.map_err(|e| SignerError::Sign(e.into()))?)
	}

	async fn public_key(&self) -> Result<<Ed25519 as Curve>::PublicKey, SignerError> {
		<Ed25519 as Curve>::PublicKey::try_from_bytes(self.inner.verifying_key.as_bytes())
			.map_err(|e| SignerError::PublicKey(e.into()))
	}
}
