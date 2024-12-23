pub mod ed25519;
use vaultrs::api::transit::KeyType;

/// Defines the needed methods for providing a definition of cryptography used with HashiCorp Vault
pub trait HashiCorpVaultCryptography {
	/// Returns the [KeyType] for the desired cryptography
	fn key_type() -> KeyType;
}
