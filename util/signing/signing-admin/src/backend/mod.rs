pub mod aws;
pub mod vault;

use anyhow::Result;
use async_trait::async_trait;
use aws::AwsBackend;
use vault::VaultBackend;

/// The trait that all signing backends must implement.
#[async_trait]
pub trait SigningBackend {
        async fn rotate_key(&self, key_id: &str) -> Result<()>;
}

/// Enum to represent the different backends.
pub enum Backend {
        Aws(AwsBackend),
        Vault(VaultBackend),
}

/// Implement the SigningBackend trait for the Backend enum.
#[async_trait]
impl SigningBackend for Backend {
        async fn rotate_key(&self, key_id: &str) -> Result<()> {
                match self {
                        Backend::Aws(aws) => aws.rotate_key(key_id).await,
                        Backend::Vault(vault) => vault.rotate_key(key_id).await,
                }
        }
}
