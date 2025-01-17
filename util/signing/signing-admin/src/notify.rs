use anyhow::Result;
use reqwest::Client;
use serde_json;

/// Notifies the application by sending the public key to the specified endpoint.
pub async fn notify_application(app_url: &str, public_key: &[u8]) -> Result<()> {
        let endpoint = format!("{}/public_key/set", app_url);
        println!("Notifying application at {} with public key {:?}", endpoint, public_key);

        let client = Client::new();

        let response = client
                .post(&endpoint)
                .json(&serde_json::json!({
                        "public_key": public_key, // Send raw byte array directly
                }))
                .send()
                .await?;

        if !response.status().is_success() {
                return Err(anyhow::anyhow!(
                        "Failed to notify application. Status: {}, Body: {:?}",
                        response.status(),
                        response.text().await?
                ));
        }

        println!("Successfully notified the application.");
        Ok(())
}

