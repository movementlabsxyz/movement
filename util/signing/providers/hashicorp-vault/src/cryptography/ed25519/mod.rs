use crate::cryptography::HashiCorpVaultCryptography;
use movement_signer::cryptography::ed25519::Ed25519;
use vaultrs::api::transit::KeyType;

impl HashiCorpVaultCryptography for Ed25519 {
	fn key_type() -> KeyType {
		KeyType::Ed25519
	}
}
