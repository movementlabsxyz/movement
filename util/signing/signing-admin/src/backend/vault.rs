use anyhow::{Context, Result};
use vaultrs::client::{VaultClient, VaultClientSettingsBuilder};
use vaultrs::transit::key::rotate;
use super::SigningBackend;

pub struct VaultBackend;

impl VaultBackend {
        pub fn new() -> Self {
                Self {}
        }

        async fn create_client(vault_url: &str, token: &str) -> Result<VaultClient> {
                let settings = VaultClientSettingsBuilder::default()
                        .address(vault_url)
                        .token(token)
                        .namespace(Some("admin".to_string()))
                        .build()
                        .context("Failed to build Vault client settings")?;
                VaultClient::new(settings).context("Failed to create Vault client")
        }
}

#[async_trait::async_trait]
impl SigningBackend for VaultBackend {
        async fn rotate_key(&self, key_id: &str) -> Result<()> {
                let vault_url = std::env::var("VAULT_URL").context("Missing VAULT_URL environment variable")?;
                let token = std::env::var("VAULT_TOKEN").context("Missing VAULT_TOKEN environment variable")?;
                let client = Self::create_client(&vault_url, &token).await?;
                rotate(&client, "transit", key_id).await.context("Failed to rotate key in Vault")
        }
}
