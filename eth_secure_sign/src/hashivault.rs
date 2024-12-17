use crate::Bytes;
use crate::Hsm;
use crate::Signature;
use anyhow::Result;
use base64::prelude::*;
use vaultrs::api::transit::requests::CreateKeyRequest;
use vaultrs::api::transit::requests::VerifySignedDataRequestBuilder;
use vaultrs::api::transit::KeyType;
use vaultrs::client::{VaultClient, VaultClientSettingsBuilder};
use vaultrs::transit::{data, key};

pub struct Vault {
    client: VaultClient,
    key: String,
}

impl Vault {
    pub async fn new(vault_url: &str, token: &str, key: String, namespace: Option<String>)  -> Result<Vault> {
        let client = VaultClient::new(
            VaultClientSettingsBuilder::default()
                .address(vault_url)
                .token(token)
                .namespace(namespace)
                .build()?
        )?;

        // Create key
        key::create(
            &client,
            "transit",
            &key,
            Some(CreateKeyRequest::builder()
               .key_type(KeyType::Ed25519)),
        ).await?;


        Ok(Vault {
            client,
            key,
        })
    }

    pub async fn create_key(&self, name: &str) -> Result<()> {
        key::create(
            &self.client,
            "transit",
            name,
            Some(CreateKeyRequest::builder()
               .key_type(KeyType::Ed25519)),
        ).await?;
        Ok(())
    }
}

#[async_trait::async_trait]
impl Hsm for Vault {
    async fn sign(&self, message: Bytes) -> Result<Signature> {
        let input = BASE64_STANDARD.encode(message.0);
        let result = data::sign(
                &self.client,
                "transit",
                &self.key,
                &input,
                None,
            )
            .await?;
        Ok(Signature(Bytes(BASE64_STANDARD.decode(&result.signature[9..])?)))
    }
    async fn verify(&self, message: Bytes, signature: Signature) -> Result<bool> {
        let input = BASE64_STANDARD.encode(message.0);
        let sig_encoded = format!("vault:v1:{}", BASE64_STANDARD.encode(signature.0 .0));

        let mut verif_builder = VerifySignedDataRequestBuilder::default();
        let verif_builder = verif_builder.mount("transit").name(&self.key).input(&input).signature(&sig_encoded);
    
        let response = data::verify(
            &self.client,
            "transit",
            &self.key,
            &input,
            Some(verif_builder),
        )
        .await?;
    
        Ok(response.valid) // Return the `valid` field from the response        
    }
}
