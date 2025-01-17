use anyhow::{Context, Result};
use signing_admin::{aws::AwsKey, key_manager::KeyManager, notify::notify_application, vault::VaultKey};

pub async fn rotate_key(
        canonical_string: String,
        application_url: String,
        backend: String,
) -> Result<()> {
        let key_manager: Box<dyn KeyManager<PublicKey = Vec<u8>>> = match backend.as_str() {
                "vault" => Box::new(VaultKey::new()),
                "aws" => {
                        Box::new(AwsKey::new())
                },
                _ => return Err(anyhow::anyhow!("Unsupported backend: {}", backend)),
        };

        let new_key_id = key_manager.rotate_key(&canonical_string).await?;
        println!("Key rotated. New Key ID: {}", new_key_id);

        let new_public_key = key_manager.fetch_public_key(&new_key_id).await?;
        println!("Retrieved public key: {:?}", new_public_key);

        notify_application(&application_url, &new_public_key).await?;
        println!("Application updated with new public key: {:?}", new_public_key);

        Ok(())
}
