use crate::cryptography::Curve;
use crate::{Bytes, PublicKey, Signature, VerifierError, VerifierOperations};
use anyhow::Context;
use k256::ecdsa::{self, VerifyingKey};
use k256::pkcs8::DecodePublicKey;
use ring_compat::signature::Verifier;

/// The secp256k1 elliptic curve.
#[derive(Debug, Clone)]
pub struct Secp256k1;

impl Curve for Secp256k1 {}

/// Built-in verifier for secp256k1.
#[async_trait::async_trait]
impl VerifierOperations<Secp256k1> for Secp256k1 {
	async fn verify(
		&self,
		message: Bytes,
		signature: Signature,
		public_key: PublicKey,
	) -> Result<bool, VerifierError> {
		let verifying_key = VerifyingKey::from_public_key_der(&public_key.0 .0)
			.context("Failed to create verifying key")
			.map_err(|e| VerifierError::Verify(e.to_string()))?;

		let signature = ecdsa::Signature::from_der(&signature.0 .0)
			.context("Failed to create signature")
			.map_err(|e| VerifierError::Verify(e.to_string()))?;

		match verifying_key.verify(message.0.as_slice(), &signature) {
			Ok(_) => Ok(true),
			Err(e) => {
				println!("Error verifying signature: {:?}", e);
				Ok(false)
			}
		}
	}
}
