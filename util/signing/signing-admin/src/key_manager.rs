use anyhow::Result;
use movement_signer::Signing;
use super::application::Application;
use super::backend::SigningBackend;

pub struct KeyManager<A, B> {
        pub application: A,
        pub backend: B,
}

impl<A, B> KeyManager<A, B>
where
        A: Application,
        B: SigningBackend,
{
        pub fn new(application: A, backend: B) -> Self {
                Self { application, backend }
        }

        pub async fn create_key(&self, key_id: &str) -> Result<String> {
                self.backend.create_key(key_id).await
        }

        pub async fn rotate_key(&self, new_key_id: &str) -> Result<()> {
                self.backend.rotate_key(new_key_id).await
        }

        pub async fn notify_application(&self, public_key: Vec<u8>) -> Result<()> {
                self.application.notify_public_key(public_key).await
        }
}

