use anyhow::{Context, Result};
use base64;
use vaultrs::api::transit::responses::ReadKeyData;
use vaultrs::client::{VaultClient, VaultClientSettingsBuilder};
use vaultrs::transit::key::{read, rotate};
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

        async fn fetch_public_key(&self, canonical_string: &str) -> Result<Self::PublicKey> {
                let vault_url = std::env::var("VAULT_URL").context("Missing VAULT_URL environment variable")?;
                let token = std::env::var("VAULT_TOKEN").context("Missing VAULT_TOKEN environment variable")?;
                let client = Self::create_client(&vault_url, &token).await?;

                println!("Fetching public key from Vault for '{}'", canonical_string);

                let mount_path = "transit";

                let key_metadata = read(&client, mount_path, canonical_string)
                        .await
                        .context("Failed to fetch key metadata from Vault")?;

                match key_metadata.keys {
                        ReadKeyData::Asymmetric(keys) => {
                                if let Some(public_key_b64) = keys
                                        .iter()
                                        .max_by_key(|(version, _)| version.parse::<u64>().unwrap_or(0))
                                        .and_then(|(_, key_data)| Some(key_data.public_key.clone()))
                                {
                                        let decoded_key = base64::decode(&public_key_b64)
                                                .context("Failed to decode base64 public key")?;
                                        println!("Successfully fetched public key: {:?}", decoded_key);
                                        return Ok(decoded_key);
                                }
                        }
                        ReadKeyData::Symmetric(_) => {
                                anyhow::bail!("Public key cannot be fetched for symmetric keys");
                        }
                }

                anyhow::bail!("Public key not found in Vault response")
        }
}
