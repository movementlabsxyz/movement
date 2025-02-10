use crate::{Bytes, PublicKey, Signature};

#[async_trait::async_trait]
pub trait LocalVerifier {
	/// Verifies a signature for a given message and public key.
	async fn verify(
		message: Bytes,
		public_key: PublicKey,
		signature: Signature,
	) -> Result<bool, anyhow::Error>;
}

pub mod secp256k1 {
	use super::*;
	use crate::cryptography::Secp256k1;
	use anyhow::Context;
	use k256::ecdsa::{self, VerifyingKey};
	use k256::pkcs8::DecodePublicKey;
	use ring_compat::signature::Verifier;

	#[async_trait::async_trait]
	impl LocalVerifier for Secp256k1 {
		async fn verify(
			message: Bytes,
			public_key: PublicKey,
			signature: Signature,
		) -> Result<bool, anyhow::Error> {
			let verifying_key = VerifyingKey::from_public_key_der(&public_key.0 .0)
				.context("Failed to create verifying key")?;

			let signature = ecdsa::Signature::from_der(&signature.0 .0)
				.context("Failed to create signature")?;

			match verifying_key.verify(message.0.as_slice(), &signature) {
				Ok(_) => Ok(true),
				Err(e) => {
					println!("Error verifying signature: {:?}", e);
					Ok(false)
				}
			}
		}
	}
}

pub mod ed25519 {

	use super::*;
	use crate::cryptography::Ed25519;
	use anyhow::Context;
	use ring_compat::signature::{
		ed25519::{self, VerifyingKey},
		Verifier,
	};

	#[async_trait::async_trait]
	impl LocalVerifier for Ed25519 {
		async fn verify(
			message: Bytes,
			public_key: PublicKey,
			signature: Signature,
		) -> Result<bool, anyhow::Error> {
			let verifying_key = VerifyingKey::from_slice(public_key.0 .0.as_slice())
				.context("Failed to create verifying key")?;

			let signature = ed25519::Signature::from_slice(signature.0 .0.as_slice())
				.context("Failed to create signature")?;

			Ok(verifying_key.verify(message.0.as_slice(), &signature).is_ok())
		}
	}
}
