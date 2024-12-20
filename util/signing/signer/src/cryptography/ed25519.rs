use crate::cryptography::Curve;
use crate::{Bytes, PublicKey, Signature, SignerError, VerifierOperations};
use anyhow::Context;
use ring_compat::signature::{
	ed25519::{self, VerifyingKey},
	Verifier,
};

/// The Ed25519 curve.
#[derive(Debug, Clone)]
pub struct Ed25519;

impl Curve for Ed25519 {}

/// Built-in verifier for Ed25519.
#[async_trait::async_trait]
impl VerifierOperations<Ed25519> for PublicKey<Ed25519> {
	async fn verify(
		&self,
		message: Bytes,
		signature: Signature,
		public_key: PublicKey<Ed25519>,
	) -> Result<bool, SignerError> {
		let verifying_key = VerifyingKey::from_slice(public_key.data.0.as_slice())
			.context("Failed to create verifying key")
			.map_err(|e| SignerError::Verify(e.to_string()))?;

		let signature = ed25519::Signature::from_slice(signature.data.0.as_slice())
			.context("Failed to create signature")
			.map_err(|e| SignerError::Verify(e.to_string()))?;

		Ok(verifying_key.verify(message.0.as_slice(), &signature).is_ok())
	}
}
