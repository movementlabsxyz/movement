use crate::cryptography::Curve;
use crate::{Verify, VerifyError};
use anyhow::Context;
use ed25519_dalek::ed25519::signature::hazmat::PrehashVerifier;
use k256::ecdsa::{self};

/// The secp256k1 elliptic curve.
#[derive(Debug, Clone, Copy)]
pub struct Secp256k1;

// Public Key in sec1 format. First byte is a 4 then the key in bytes.
fixed_size!(pub struct PublicKey([u8; 65]));
// ECDSA Signature noralized: 64 bytes, the first 32 bytes are the r value, the second 32 bytes the s value.
fixed_size!(pub struct Signature([u8; 64]));

impl Curve for Secp256k1 {
	type PublicKey = PublicKey;
	type Signature = Signature;
}

/// Built-in verifier for secp256k1.
impl Verify<Secp256k1> for Secp256k1 {
	fn verify(
		&self,
		message: &[u8],
		signature: &Signature,
		public_key: &PublicKey,
	) -> Result<bool, VerifyError> {
		let verifying_key = ecdsa::VerifyingKey::from_sec1_bytes(&public_key.0)
			.context("Failed to create verifying key")
			.map_err(|e| VerifyError(e.into()))?;

		let signature = ecdsa::Signature::from_slice(&signature.0)
			.context("Failed to create signature")
			.map_err(|e| VerifyError(e.into()))?;

		Ok(verifying_key.verify_prehash(message, &signature).is_ok())
	}
}
