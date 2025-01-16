use anyhow::Result;

/// Trait for managing signing keys
pub trait KeyManager {
        type PublicKey;

        /// Rotate a key and return the new key ID
        fn rotate_key(&self, canonical_string: &str) -> Result<String>;

        /// Fetch the public key for a given key ID
        fn fetch_public_key(&self, key_id: &str) -> Result<Self::PublicKey>;
}
