use crate::cryptography::Curve;
use crate::{DigestError, Digester, Verify, VerifyError};
use anyhow::Context;
use ed25519_dalek::Verifier as _;
use k256::ecdsa;
use serde::{Deserialize, Serialize};
use sha2::Digest as _;
use tracing::info;

/// The secp256k1 elliptic curve.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Secp256k1;

fixed_size!(pub struct PublicKey([u8; 65])); // Compressed public key
fixed_size!(pub struct Signature([u8; 64]));
fixed_size!(pub struct Digest([u8; 32]));

impl Curve for Secp256k1 {
	type PublicKey = PublicKey;
	type Signature = Signature;
	type Digest = Digest;
}

/// Built-in verifier for secp256k1.
impl Verify<Secp256k1> for Secp256k1 {
	fn verify(
		message: &[u8],
		signature: &Signature,
		public_key: &PublicKey,
	) -> Result<bool, VerifyError> {
		// sec1 encoding is the same as uncompressed public key
		let verifying_key = ecdsa::VerifyingKey::from_sec1_bytes(&public_key.0)
			.context("Failed to create verifying key")
			.map_err(|e| VerifyError(e.into()))?;

		let signature = ecdsa::Signature::from_slice(&signature.0)
			.context("Failed to create signature")
			.map_err(|e| VerifyError(e.into()))?;

		match verifying_key.verify(message, &signature) {
			Ok(_) => Ok(true),
			Err(e) => {
				info!("Failed to verify signature: {:?}", e);
				Ok(false)
			}
		}
	}
}

/// Built-in digest for secp256k1.
impl Digester<Secp256k1> for Secp256k1 {
	fn digest(message: &[u8]) -> Result<Digest, DigestError> {
		let digest = sha2::Sha256::digest(message);
		let mut result = [0u8; 32];
		result.copy_from_slice(&digest);
		Ok(Digest(result))
	}
}
