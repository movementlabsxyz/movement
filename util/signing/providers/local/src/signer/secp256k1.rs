use crate::signer::LocalSigner;
use ecdsa::{SigningKey, VerifyingKey};
use movement_signer::{cryptography::secp256k1::Secp256k1, SignerError};

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

	/// Constructs a new [LocalSigner] from a hex string.
	pub fn from_signing_key_hex(hex: &str) -> Result<Self, SignerError> {
		let bytes = hex::decode(hex).map_err(|e| {
			SignerError::Decode(format!("failed to decode hex string: {}", e).into())
		})?;
		Self::from_signing_key_bytes(&bytes)
	}
}
