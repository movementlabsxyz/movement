use crate::cryptography::{CryptoMaterialError, Curve};
use crate::{Verify, VerifyError};
use anyhow::Context;
use ed25519_dalek::Verifier as _;

/// The Ed25519 curve.
#[derive(Debug, Clone, Copy)]
pub struct Ed25519;

fixed_size!(pub struct PublicKey([u8; 32]));
fixed_size!(pub struct Signature([u8; 64]));

impl Curve for Ed25519 {
	type PublicKey = PublicKey;
	type Signature = Signature;
}

/// Built-in verifier for Ed25519.
impl Verify<Ed25519> for Ed25519 {
	fn verify(
		&self,
		message: &[u8],
		signature: &Signature,
		public_key: &PublicKey,
	) -> Result<bool, VerifyError> {
		let verifying_key = ed25519_dalek::VerifyingKey::from_bytes(&public_key.0)
			.context("failed to create verifying key")
			.map_err(|e| VerifyError(e.into()))?;

		let signature = ed25519_dalek::Signature::from_bytes(&signature.0);

		Ok(verifying_key.verify(message, &signature).is_ok())
	}
}

impl TryFrom<[u8; 32]> for PublicKey {
	type Error = CryptoMaterialError;

	#[inline]
	fn try_from(bytes: [u8; 32]) -> Result<Self, Self::Error> {
		PublicKey::try_from(&bytes)
	}
}

impl TryFrom<&[u8; 32]> for PublicKey {
	type Error = CryptoMaterialError;

	fn try_from(bytes: &[u8; 32]) -> Result<Self, Self::Error> {
		let key = ed25519_dalek::VerifyingKey::from_bytes(bytes)
			.map_err(|e| CryptoMaterialError(e.into()))?;
		Ok(PublicKey(key.to_bytes()))
	}
}

// Following the example given by ed25519, we allow any 64 bytes to construct a
// (possibly invalid) signature.
impl From<[u8; 64]> for Signature {
	#[inline]
	fn from(bytes: [u8; 64]) -> Self {
		Signature(bytes)
	}
}
impl From<&[u8; 64]> for Signature {
	#[inline]
	fn from(bytes: &[u8; 64]) -> Self {
		Signature(*bytes)
	}
}
