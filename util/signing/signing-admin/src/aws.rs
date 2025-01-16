use anyhow::{Context, Result};
use serde_json::Value;
use base64;
use simple_asn1::{ASN1Block, from_der};
use super::key_manager::KeyManager;

/// Struct for managing AWS KMS keys
pub struct AwsKey;

impl AwsKey {
        pub fn new() -> Self {
                Self
        }

        /// Creates a new AWS KMS key
        fn create_key(&self) -> Result<String> {
                println!("Creating a new AWS KMS key");
        
                // Generate a random tag for uniqueness
                let random_tag = format!("tag-{}", uuid::Uuid::new_v4());
        
                let output = std::process::Command::new("aws")
                        .args([
                                "kms",
                                "create-key",
                                "--description",
                                "Key for signing and verification",
                                "--key-usage",
                                "SIGN_VERIFY",
                                "--customer-master-key-spec",
                                "ECC_SECG_P256K1",
                                "--tags",
                                &format!("TagKey=unique_id,TagValue={}", random_tag),
                                "--output",
                                "json",
                        ])
                        .output()
                        .context("Failed to execute AWS CLI to create key")?;
        
                if output.status.success() {
                        let stdout = String::from_utf8_lossy(&output.stdout);
                        let response: serde_json::Value =
                                serde_json::from_str(&stdout).context("Failed to parse AWS response")?;
        
                        if let Some(key_id) = response["KeyMetadata"]["KeyId"].as_str() {
                                println!("Successfully created new key with ID: {}", key_id);
                                Ok(key_id.to_string())
                        } else {
                                anyhow::bail!("Key ID not found in AWS response");
                        }
                } else {
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        anyhow::bail!("Failed to create key: {}", stderr);
                }
        }

        fn extract_raw_public_key(der: &[u8]) -> Result<Vec<u8>> {
                let asn1_blocks = from_der(der).context("Failed to parse DER-encoded public key")?;

                if let Some(ASN1Block::Sequence(_, blocks)) = asn1_blocks.get(0) {
                        if let Some(ASN1Block::BitString(_, _, key_bytes)) = blocks.get(1) {
                                // Ensure the key is in uncompressed format and strip the prefix
                                if key_bytes.len() == 65 && key_bytes[0] == 4 {
                                        return Ok(key_bytes[1..].to_vec()); // Return full X and Y coordinates
                                } else {
                                        return Err(anyhow::anyhow!(
                                                "Unexpected public key format or length: {:?}",
                                                key_bytes
                                        ));
                                }
                        }
                }

                Err(anyhow::anyhow!("Failed to extract raw public key from DER"))
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

                println!("Creating a new key to rotate alias: {}", full_alias);

                // Create a new key
                let new_key_id = self.create_key().context("Failed to create a new key for rotation")?;

                println!(
                        "Rotating AWS KMS alias '{}' to point to new key ID '{}'",
                        full_alias, new_key_id
                );

                // Update the alias to point to the new key
                let output = std::process::Command::new("aws")
                        .args([
                                "kms",
                                "update-alias",
                                "--alias-name",
                                &full_alias,
                                "--target-key-id",
                                &new_key_id,
                        ])
                        .output()
                        .context("Failed to execute AWS CLI to update alias")?;

                if output.status.success() {
                        println!("Successfully rotated key for alias: {}", full_alias);
                        Ok(new_key_id)
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
                        let response: Value =
                                serde_json::from_str(&stdout).context("Failed to parse AWS response")?;

                        if let Some(public_key_b64) = response["PublicKey"].as_str() {
                                let der_encoded_key = base64::decode(public_key_b64)
                                        .context("Failed to decode base64 public key")?;
                                println!("Successfully fetched DER-encoded public key: {:?}", der_encoded_key);

                                // Convert DER-encoded public key to raw format
                                let raw_public_key = Self::extract_raw_public_key(&der_encoded_key)
                                        .context("Failed to extract raw public key from DER")?;
                                println!("Successfully extracted raw public key: {:?}", raw_public_key);

                                Ok(raw_public_key)
                        } else {
                                anyhow::bail!("Public key not found in AWS response");
                        }
                } else {
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        anyhow::bail!("Failed to fetch public key from AWS: {}", stderr);
                }
        }
}
