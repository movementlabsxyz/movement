use crate::cryptography::Curve;
use crate::{DigestError, Digester, Verify, VerifyError};
use anyhow::Context;
use ed25519_dalek::Verifier as _;
use serde::{Deserialize, Serialize};
use sha2::Digest as _;

/// The Ed25519 curve.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Ed25519;

fixed_size!(pub struct PublicKey([u8; 32]));
fixed_size!(pub struct Signature([u8; 64]));
fixed_size!(pub struct Digest([u8; 64]));

impl Curve for Ed25519 {
	type PublicKey = PublicKey;
	type Signature = Signature;
	type Digest = Digest;
}

/// Built-in verifier for Ed25519.
impl Verify<Ed25519> for Ed25519 {
	fn verify(
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

/// Built-in digest for Ed25519.
impl Digester<Ed25519> for Ed25519 {
	fn digest(message: &[u8]) -> Result<Digest, DigestError> {
		let digest = sha2::Sha512::digest(message);
		let mut result = [0u8; 64];
		result.copy_from_slice(&digest);
		Ok(Digest(result))
	}
}
