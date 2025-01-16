use anyhow::{Context, Result};
use super::key_manager::KeyManager;

/// Struct for managing AWS KMS keys
pub struct AwsKey {
        key_id: String,
}

impl AwsKey {
        pub fn new(key_id: String) -> Self {
                Self { key_id }
        }
}

impl KeyManager for AwsKey {
        type PublicKey = Vec<u8>;

        fn rotate_key(&self, alias: &str) -> Result<String> {
                // Ensure the alias starts with "alias/"
                let full_alias = if alias.starts_with("alias/") {
                        alias.to_string()
                } else {
                        format!("alias/{}", alias)
                };

                println!("Rotating AWS KMS key for alias: {}", full_alias);

                let output = std::process::Command::new("aws")
                        .args([
                                "kms",
                                "update-alias",
                                "--alias-name",
                                &full_alias,
                                "--target-key-id",
                                &self.key_id,
                        ])
                        .output()
                        .context("Failed to execute AWS CLI")?;

                if output.status.success() {
                        println!("Successfully rotated key for alias: {}", full_alias);
                        Ok(self.key_id.clone())
                } else {
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        anyhow::bail!("Failed to rotate key: {}", stderr);
                }
        }

        fn fetch_public_key(&self, key_id: &str) -> Result<Self::PublicKey> {
                println!("Fetching public key for AWS Key ID: {}", key_id);

                let output = std::process::Command::new("aws")
                        .args([
                                "kms",
                                "get-public-key",
                                "--key-id",
                                key_id,
                                "--output",
                                "json",
                        ])
                        .output()
                        .context("Failed to execute AWS CLI to get public key")?;

                if output.status.success() {
                        let stdout = String::from_utf8_lossy(&output.stdout);
                        let response: serde_json::Value =
                                serde_json::from_str(&stdout).context("Failed to parse AWS response")?;

                        if let Some(public_key_b64) = response["PublicKey"].as_str() {
                                let public_key = base64::decode(public_key_b64)
                                        .context("Failed to decode base64 public key")?;
                                println!("Successfully fetched public key: {:?}", public_key);
                                Ok(public_key)
                        } else {
                                anyhow::bail!("Public key not found in AWS response");
                        }
                } else {
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        anyhow::bail!("Failed to fetch public key from AWS: {}", stderr);
                }
        }
}

