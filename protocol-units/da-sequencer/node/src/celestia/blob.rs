use crate::DaSequencerError;
use movement_signer::cryptography::ed25519::{Ed25519, PublicKey, Signature};
use movement_signer::{SignerError, Signing, Verify};
use movement_types::block;
use serde::{Deserialize, Serialize};

/// The blob format that is stored in Celestia DA.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CelestiaBlob {
	// serialized data
	data: Vec<u8>,
	// signer's public key
	public_key: PublicKey,
	// ed25519 signature
	signature: Signature,
}

impl CelestiaBlob {
	pub async fn sign<S>(block_ids: &[block::Id], signer: &S) -> Result<Self, SignerError>
	where
		S: Signing<Ed25519>,
	{
		let public_key = signer.public_key().await?;
		// Serialization shoud never fail
		let data = bcs::to_bytes(block_ids).unwrap();
		let signature = signer.sign(&data).await?;
		Ok(CelestiaBlob { data, public_key, signature })
	}

	pub fn public_key(&self) -> PublicKey {
		self.public_key
	}

	pub fn verified_block_ids(&self) -> Result<Vec<block::Id>, DaSequencerError> {
		Ed25519::verify(&self.data, &self.signature, &self.public_key)
			.map_err(|_| DaSequencerError::InvalidSignature)?;
		let block_ids = bcs::from_bytes(&self.data)
			.map_err(|e| DaSequencerError::Deserialization(e.to_string()))?;
		Ok(block_ids)
	}
}
