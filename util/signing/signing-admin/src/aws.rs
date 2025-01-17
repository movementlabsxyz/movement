use anyhow::{Context, Result};
use aws_config;
use aws_sdk_kms::Client as KmsClient;
use aws_sdk_kms::types::Tag;
use simple_asn1::{ASN1Block, from_der};
use super::key_manager::KeyManager;

pub struct AwsKey;

impl AwsKey {
        pub fn new() -> Self {
                Self {}
        }

        /// Helper function to create an `AwsKmsClient`
        pub async fn create_client() -> Result<KmsClient> {
                let aws_config = aws_config::load_from_env().await;
                Ok(KmsClient::new(&aws_config))
        }
            

        /// Creates a new AWS KMS key
        pub async fn create_key(client: &KmsClient) -> Result<String> {
                println!("Creating a new AWS KMS key");

                // Generate a random tag for uniqueness
                let random_tag = format!("tag-{}", uuid::Uuid::new_v4());

                // Create the key with the AWS SDK
                let response = client
                        .create_key()
                        .description("Key for signing and verification")
                        .key_usage(aws_sdk_kms::types::KeyUsageType::SignVerify)
                        .customer_master_key_spec(aws_sdk_kms::types::CustomerMasterKeySpec::EccSecgP256K1)
                        .tags(
                                Tag::builder()
                                    .tag_key("unique_id")
                                    .tag_value(&random_tag)
                                    .build()
                                    .context("Failed to build AWS KMS tag")?,
                            )
                        .send()
                        .await
                        .context("Failed to create key with AWS KMS")?;

                if let Some(key_id) = response.key_metadata().and_then(|meta| Some(meta.key_id())) {
                        println!("Successfully created new key with ID: {}", key_id);
                        Ok(key_id.to_string())
                } else {
                        anyhow::bail!("Key ID not found in AWS response");
                }
        }

        /// Extract raw public key from DER-encoded key
        pub fn extract_raw_public_key(der: &[u8]) -> Result<Vec<u8>> {
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

#[async_trait::async_trait]
impl KeyManager for AwsKey {
        type PublicKey = Vec<u8>;

        /// Rotate a key in AWS KMS
        async fn rotate_key(&self, alias: &str) -> Result<String> {
                let client = Self::create_client().await?;
                let full_alias = if alias.starts_with("alias/") {
                        alias.to_string()
                } else {
                        format!("alias/{}", alias)
                };

                println!("Creating a new key to rotate alias: {}", full_alias);

                let new_key_id = Self::create_key(&client)
                        .await
                        .context("Failed to create a new key for rotation")?;

                println!(
                        "Rotating AWS KMS alias '{}' to point to new key ID '{}'",
                        full_alias, new_key_id
                );

                client
                        .update_alias()
                        .alias_name(&full_alias)
                        .target_key_id(&new_key_id)
                        .send()
                        .await
                        .context("Failed to update alias")?;

                println!("Successfully rotated key for alias: {}", full_alias);
                Ok(new_key_id)
        }
}
