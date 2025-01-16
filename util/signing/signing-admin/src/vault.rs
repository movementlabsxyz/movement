use anyhow::{Context, Result};
use super::key_manager::KeyManager;

pub struct VaultKey;

impl VaultKey {
        pub fn new() -> Self {
                Self {}
        }
}

impl KeyManager for VaultKey {
        type PublicKey = Vec<u8>;

        fn rotate_key(&self, canonical_string: &str) -> Result<String> {
                println!("Rotating key in Vault for '{}'", canonical_string);

                let output = std::process::Command::new("vault")
                        .args([
                                "write",
                                "-f",
                                &format!("transit/keys/{}/rotate", canonical_string),
                        ])
                        .output()
                        .context("Failed to execute Vault CLI")?;

                if output.status.success() {
                        println!("Successfully rotated key for '{}'", canonical_string);
                        // Return the canonical string since Vault rotation doesn't generate a new key ID
                        Ok(canonical_string.to_string())
                } else {
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        anyhow::bail!("Failed to rotate key in Vault: {}", stderr);
                }
        }

        fn fetch_public_key(&self, canonical_string: &str) -> Result<Vec<u8>> {
                println!("Fetching public key from Vault for '{}'", canonical_string);

                let output = std::process::Command::new("vault")
                        .args([
                                "read",
                                "-format=json",
                                &format!("transit/keys/{}", canonical_string),
                        ])
                        .output()
                        .context("Failed to execute Vault CLI")?;

                if output.status.success() {
                        let stdout = String::from_utf8_lossy(&output.stdout);

                        let response: serde_json::Value = serde_json::from_str(&stdout)
                                .context("Failed to parse Vault response")?;

                        // Retrieve the latest key's public key by finding the highest version
                        if let Some(public_key) = response["data"]["keys"]
                                .as_object()
                                .and_then(|keys| {
                                        keys.iter()
                                                .max_by_key(|(version, _)| version.parse::<u64>().unwrap_or(0))
                                                .and_then(|(_, key_data)| key_data["public_key"].as_str())
                                })
                        {
                                let decoded_key = base64::decode(public_key)
                                        .context("Failed to decode base64 public key")?;
                                println!("Successfully fetched public key: {:?}", decoded_key);
                                Ok(decoded_key)
                        } else {
                                anyhow::bail!("Public key not found in Vault response");
                        }
                } else {
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        anyhow::bail!("Failed to fetch public key from Vault: {}", stderr);
                }
        }
}
