use anyhow::Result;

#[async_trait::async_trait]
pub trait Application {
        async fn notify_public_key(&self, public_key: Vec<u8>) -> Result<()>;
}

pub struct HttpApplication {
        app_url: String,
}

impl HttpApplication {
        pub fn new(app_url: String) -> Self {
                Self { app_url }
        }
}

#[async_trait::async_trait]
impl Application for HttpApplication {
        async fn notify_public_key(&self, public_key: Vec<u8>) -> Result<()> {
                let endpoint = format!("{}/public_key/set", self.app_url);
                println!("Notifying application at {} with public key {:?}", endpoint, public_key);

                let client = reqwest::Client::new();
                let response = client
                        .post(&endpoint)
                        .json(&serde_json::json!({ "public_key": public_key }))
                        .send()
                        .await?;

                if !response.status().is_success() {
                        anyhow::bail!(
                                "Failed to notify application. Status: {}, Body: {:?}",
                                response.status(),
                                response.text().await?
                        );
                }

                println!("Successfully notified the application.");
                Ok(())
        }
}
