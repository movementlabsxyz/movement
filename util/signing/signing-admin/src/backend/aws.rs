use anyhow::{Context, Result};
use aws_config;
use aws_sdk_kms::{Client as KmsClient};
use aws_sdk_kms::types::Tag;
use super::SigningBackend;

pub struct AwsBackend;

impl AwsBackend {
        pub fn new() -> Self {
                Self {}
        }

        async fn create_client() -> Result<KmsClient> {
                let aws_config = aws_config::load_from_env().await;
                Ok(KmsClient::new(&aws_config))
        }

        async fn create_key(client: &KmsClient) -> Result<String> {
                let response = client
                        .create_key()
                        .description("Key for signing and verification")
                        .key_usage(aws_sdk_kms::types::KeyUsageType::SignVerify)
                        .customer_master_key_spec(aws_sdk_kms::types::CustomerMasterKeySpec::EccSecgP256K1)
                        .tags(
                                Tag::builder()
                                        .tag_key("unique_id")
                                        .tag_value("tag")
                                        .build()
                                        .context("Failed to build AWS KMS tag")?,
                        )
                        .send()
                        .await
                        .context("Failed to create key with AWS KMS")?;

                response
                        .key_metadata()
                        .and_then(|meta| Some(meta.key_id().to_string()))
                        .context("Failed to retrieve key ID from AWS response")
        }
}

#[async_trait::async_trait]
impl SigningBackend for AwsBackend {
        async fn rotate_key(&self, key_id: &str) -> Result<()> {
                let client = Self::create_client().await?;
                
                // Ensure the key_id starts with "alias/"
                let full_alias = if key_id.starts_with("alias/") {
                        key_id.to_string()
                } else {
                        format!("alias/{}", key_id)
                };

                let new_key_id = Self::create_key(&client).await?;
                client
                        .update_alias()
                        .alias_name(&full_alias)
                        .target_key_id(&new_key_id)
                        .send()
                        .await
                        .context("Failed to update AWS KMS alias")?;
                
                Ok(())
        }
}
