use crate::cryptography::Ed25519;
use vaultrs::api::transit::KeyType;

/// Defines the needed methods for providing a definition of cryptography used with HashiCorp Vault
pub trait HashiCorpVaultCryptography {
	/// Returns the [KeyType] for the desired cryptography
	fn key_type() -> KeyType;
}

impl HashiCorpVaultCryptography for Ed25519 {
	fn key_type() -> KeyType {
		KeyType::Ed25519
	}
}
