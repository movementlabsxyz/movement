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

        pub async fn rotate_key(&self, key_id: &str) -> Result<()> {
                self.backend.rotate_key(key_id).await
        }

}
