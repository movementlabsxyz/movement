use anyhow::{Context, Result};
use vaultrs::client::{VaultClient, VaultClientSettingsBuilder};
use vaultrs::transit::key::rotate;
use super::key_manager::KeyManager;

pub struct VaultKey;

impl VaultKey {
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
impl KeyManager for VaultKey {
        type PublicKey = Vec<u8>;

        async fn rotate_key(&self, canonical_string: &str) -> Result<String> {
                let vault_url = std::env::var("VAULT_URL").context("Missing VAULT_URL environment variable")?;
                let token = std::env::var("VAULT_TOKEN").context("Missing VAULT_TOKEN environment variable")?;

                println!("Using Vault URL: {}", vault_url);
                println!("Using Vault Token: {} (truncated)", &token[..std::cmp::min(token.len(), 6)]);

                let client = Self::create_client(&vault_url, &token).await?;
                println!("Vault client successfully created.");

                println!("Attempting to rotate key for canonical string: {}", canonical_string);
                let mount_path = "transit";
                println!("Using mount path: {}", mount_path);

                // Debugging the request path
                println!(
                    "Full key rotation path: {}/keys/{}/rotate",
                    mount_path, canonical_string
                );

                rotate(&client, mount_path, canonical_string)
                        .await
                        .map_err(|err| {
                            eprintln!("Error during rotation: {:?}", err);
                            err
                        })
                        .context("Failed to rotate key in Vault")?;

                println!("Successfully rotated key for '{}'", canonical_string);
                Ok(canonical_string.to_string())
        }
}
