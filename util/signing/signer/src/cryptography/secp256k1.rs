use crate::cryptography::Curve;
use crate::{VerifierError, VerifierOperations};
use anyhow::Context;
use k256::ecdsa;
use ring_compat::signature::Verifier;

/// The secp256k1 elliptic curve.
#[derive(Debug, Clone)]
pub struct Secp256k1;

fixed_size!(pub struct PublicKey([u8; 32]));
fixed_size!(pub struct Signature([u8; 64]));

impl Curve for Secp256k1 {
	type PublicKey = PublicKey;
	type Signature = Signature;
}

/// Built-in verifier for secp256k1.
#[async_trait::async_trait]
impl VerifierOperations<Secp256k1> for Secp256k1 {
	async fn verify(
		&self,
		message: &[u8],
		signature: &Signature,
		public_key: &PublicKey,
	) -> Result<bool, VerifierError> {
		let verifying_key = ecdsa::VerifyingKey::from_sec1_bytes(&public_key.0)
			.context("Failed to create verifying key")
			.map_err(|e| VerifierError::Verify(e.to_string()))?;

		let signature = ecdsa::Signature::from_slice(&signature.0)
			.context("Failed to create signature")
			.map_err(|e| VerifierError::Verify(e.to_string()))?;

		Ok(verifying_key.verify(message, &signature).is_ok())
	}
}
