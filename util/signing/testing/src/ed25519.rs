use ed25519_dalek::{Signer as _, SigningKey};
use movement_signer::cryptography::ed25519::{self, Ed25519};
use movement_signer::cryptography::TryFromBytes;
use movement_signer::{SignerError, Signing};

/// In-process signer used for testing the signing API.
///
/// This signer wraps an Ed25519 private key to provide a signing service with
/// the Ed25519 elliptic curve. Because the private key is kept in process
/// memory, this signer implementation should not be used in production.
pub struct TestSigner {
	signing_key: SigningKey,
}

impl TestSigner {
	pub fn new(signing_key: SigningKey) -> Self {
		Self { signing_key }
	}
}

#[async_trait::async_trait]
impl Signing<Ed25519> for TestSigner {
	async fn sign(&self, message: &[u8]) -> Result<ed25519::Signature, SignerError> {
		let signature =
			self.signing_key.try_sign(message).map_err(|e| SignerError::Sign(e.into()))?;
		Ok(ed25519::Signature::try_from_bytes(signature.to_bytes().as_ref())
			.map_err(|e| SignerError::Internal(format!("failed to convert signature: {}", e)))?)
	}

	async fn public_key(&self) -> Result<ed25519::PublicKey, SignerError> {
		let key = self.signing_key.verifying_key();
		// The conversion should never fail because it's round-tripping
		// a valid key.
		Ok(ed25519::PublicKey::try_from_bytes(key.to_bytes().as_ref())
			.map_err(|e| SignerError::Internal(format!("failed to convert public key: {}", e)))?)
	}
}
