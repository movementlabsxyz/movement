use crate::cryptography::Curve;
use crate::{VerifierError, VerifierOperations};
use anyhow::Context;
use ring_compat::signature::{ed25519, Verifier};

/// The Ed25519 curve.
#[derive(Debug, Clone)]
pub struct Ed25519;

fixed_size!(pub struct PublicKey([u8; 32]));
fixed_size!(pub struct Signature([u8; 64]));

impl Curve for Ed25519 {
	type PublicKey = PublicKey;
	type Signature = Signature;
}

/// Built-in verifier for Ed25519.
#[async_trait::async_trait]
impl VerifierOperations<Ed25519> for Ed25519 {
	async fn verify(
		&self,
		message: &[u8],
		signature: &Signature,
		public_key: &PublicKey,
	) -> Result<bool, VerifierError> {
		let verifying_key = ed25519::VerifyingKey::from_slice(&public_key.0)
			.context("Failed to create verifying key")
			.map_err(|e| VerifierError::Verify(e.to_string()))?;

		let signature = ed25519::Signature::from_slice(&signature.0)
			.context("Failed to create signature")
			.map_err(|e| VerifierError::Verify(e.to_string()))?;

		Ok(verifying_key.verify(message, &signature).is_ok())
	}
}
