use anyhow::Result;

#[async_trait::async_trait]
pub trait KeyManager {
        type PublicKey;
        async fn rotate_key(&self, canonical_string: &str) -> Result<String>;
}
