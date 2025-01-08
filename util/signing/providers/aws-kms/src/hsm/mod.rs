use crate::cryptography::AwsKmsCryptographySpec;
use anyhow::Context;
use aws_sdk_kms::{Client as KmsClient, primitives::Blob};
use aws_sdk_ssm::Client;
use aws_types::SdkConfig;
use movement_signer::cryptography::TryFromBytes;
use movement_signer::{cryptography::Curve, SignerError, Signing};

pub mod key;

/// An AWS KMS HSM.
pub struct AwsKms<C: Curve + AwsKmsCryptographySpec> {
        client: KmsClient,
        key_id: String,
        _cryptography_marker: std::marker::PhantomData<C>,
}

impl<C> AwsKms<C>
where
        C: Curve + AwsKmsCryptographySpec,
{
        /// Creates a new AWS KMS HSM
        pub fn new(client: KmsClient, key_id: String) -> Self {
                Self { client, key_id, _cryptography_marker: std::marker::PhantomData }
        }

        /// Sets the key ID
        pub fn set_key_id(&mut self, key_id: String) {
                self.key_id = key_id;
        }

        /// Tries to create a new AWS KMS HSM from the environment
        pub async fn try_from_env() -> Result<Self, anyhow::Error> {
                // Load shared AWS configuration
                let shared_config: SdkConfig = aws_config::load_from_env().await;

                // Fetch the canonical key ID from Parameter Store
                let canonical_key = "/movement/prod/full_node/mcr_settlement/signer/weth-transfer-sign/replica-1";
                let ssm_client = Client::new(&shared_config);
                let response = ssm_client
                        .get_parameter()
                        .name(canonical_key)
                        .with_decryption(true) // Fetch the decrypted value
                        .send()
                        .await
                        .map_err(|e| anyhow::anyhow!("Failed to fetch parameter: {}", e))?;

                let key_id = response.parameter.unwrap().value.unwrap();
                println!("Fetched key ID from Parameter Store: {}", key_id);

                // Fetch AWS KMS client configuration
                let kms_client = KmsClient::new(&shared_config);

                Ok(Self::new(kms_client, key_id))
        }

        /// Creates a new key in AWS KMS matching the provided key ID.
        pub async fn create_key(self) -> Result<Self, anyhow::Error> {
                let res = self
                        .client
                        .create_key()
                        .key_spec(C::key_spec())
                        .key_usage(C::key_usage_type())
                        .send()
                        .await?;

                let key_id = res
                        .key_metadata()
                        .context("No key metadata available")?
                        .key_id()
                        .to_string();

                Ok(Self::new(self.client, key_id))
        }
}

impl<C> Signing<C> for AwsKms<C>
where
        C: Curve + AwsKmsCryptographySpec + Sync,
{
        async fn sign(&self, message: &[u8]) -> Result<C::Signature, SignerError> {
                println!(
                        "Signing request: key_id={}, algorithm={}, message={:?}",
                        &self.key_id,
                        C::signing_algorithm_spec(),
                        message
                );

                let blob = Blob::new(message);
                let request = self
                        .client
                        .sign()
                        .key_id(&self.key_id)
                        .signing_algorithm(C::signing_algorithm_spec())
                        .message(blob);

                let res = request
                        .send()
                        .await
                        .map_err(|e| SignerError::Internal(format!("Failed to sign: {}", e.to_string())))?;

                let signature = <C as Curve>::Signature::try_from_bytes(
                        res.signature()
                                .context("No signature available")
                                .map_err(|e| {
                                        SignerError::Internal(format!("Failed to convert signature: {}", e.to_string()))
                                })?
                                .as_ref(),
                )
                .map_err(|e| {
                        SignerError::Internal(format!("Failed to convert signature: {}", e.to_string()))
                })?;

                Ok(signature)
        }

        async fn public_key(&self) -> Result<C::PublicKey, SignerError> {
                let res = self
                        .client
                        .get_public_key()
                        .key_id(&self.key_id)
                        .send()
                        .await
                        .map_err(|e| SignerError::Internal(format!("Failed to get public key: {}", e.to_string())))?;

                let public_key = C::PublicKey::try_from_bytes(
                        res.public_key()
                                .context("No public key available")
                                .map_err(|e| {
                                        SignerError::Internal(format!(
                                                "Failed to convert public key: {}",
                                                e.to_string()
                                        ))
                                })?
                                .as_ref(),
                )
                .map_err(|e| {
                        SignerError::Internal(format!("Failed to convert public key: {}", e.to_string()))
                })?;

                Ok(public_key)
        }
}
