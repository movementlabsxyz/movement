use anyhow::{Context, Result};
use movement_signer::{
        cryptography::{secp256k1::Secp256k1, ed25519::Ed25519},
        Signing,
};
use signing_admin::{
        aws::AwsKey,
        key_manager::KeyManager,
        notify::notify_application,
        vault::VaultKey,
};
use movement_signer_aws_kms::hsm::AwsKms;
use movement_signer_hashicorp_vault::hsm::HashiCorpVault;

pub async fn rotate_key(
        canonical_string: String,
        application_url: String,
        backend: String,
) -> Result<()> {
        // Use KeyManager for key rotation
        let key_manager: Box<dyn KeyManager<PublicKey = Vec<u8>>> = match backend.as_str() {
                "vault" => Box::new(VaultKey::new()),
                "aws" => Box::new(AwsKey::new()),
                _ => return Err(anyhow::anyhow!("Unsupported backend: {}", backend)),
        };

        // Rotate the key using KeyManager
        let new_key_id = key_manager
                .rotate_key(&canonical_string)
                .await
                .context("Failed to rotate key")?;
        println!("Key rotated successfully. New Key ID: {}", new_key_id);

        // Fetch the public key using the Signing trait
        match backend.as_str() {
                "vault" => {
                        // Create a Vault client
                        let vault_url = std::env::var("VAULT_URL")
                                .context("Missing VAULT_URL environment variable")?;
                        let vault_token = std::env::var("VAULT_TOKEN")
                                .context("Missing VAULT_TOKEN environment variable")?;
                        let client = vaultrs::client::VaultClient::new(
                                vaultrs::client::VaultClientSettingsBuilder::default()
                                        .address(vault_url)
                                        .token(vault_token)
                                        .namespace(Some("admin".to_string())) // Adjust namespace if necessary
                                        .build()
                                        .context("Failed to build Vault client settings")?,
                        )
                        .context("Failed to create Vault client")?;

                        // Sanitize and log the key name
                        let sanitized_key_name = canonical_string.replace("/", "_");
                        println!("Attempting to read key: {}", sanitized_key_name);
                        let signer = HashiCorpVault::<Ed25519>::new(client, sanitized_key_name.clone(), "transit".to_string());
                        let public_key = signer
                                .public_key()
                                .await
                                .context("Failed to fetch Vault public key")?;
                        println!("Retrieved public key: {:?}", public_key.as_bytes());
                        
                        // Notify the application with the new public key
                        notify_application(&application_url, &public_key.as_bytes()).await?;
                }
                "aws" => {
                        // Create an AWS KMS client
                        let aws_config = aws_config::load_from_env().await;
                        let client = aws_sdk_kms::Client::new(&aws_config);
                        let signer = AwsKms::<Secp256k1>::new(client, canonical_string.clone());
                        let public_key = signer
                                .public_key()
                                .await
                                .context("Failed to fetch AWS public key")?;
                        println!("Retrieved public key: {:?}", public_key.as_bytes());
                        notify_application(&application_url, &public_key.as_bytes()).await?;
                }
                _ => return Err(anyhow::anyhow!("Unsupported backend: {}", backend)),
        }

        println!("Application successfully updated with the new public key.");
        Ok(())
}
