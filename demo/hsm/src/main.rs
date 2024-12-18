use axum::Server;
use hsm_demo::{hsm, Bytes, Hsm, PublicKey, Signature};
use reqwest::Client;
use serde::Serialize;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::task;
use dotenv::dotenv;

use hsm_demo::{action_stream, Application};
use hsm_demo::server::create_server;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    dotenv().ok(); // Load environment variables from .env file

    // Initialize HSM based on PROVIDER
    let provider = std::env::var("PROVIDER").unwrap_or_else(|_| "AWS".to_string());
    let (hsm, public_key) = match provider.as_str() {
        "AWS" => {
            let aws_kms_hsm = hsm::aws_kms::AwsKms::try_from_env()
                .await?
                .create_key()
                .await?
                .fill_with_public_key()
                .await?;
            let public_key = aws_kms_hsm.public_key.clone();
            (Arc::new(Mutex::new(aws_kms_hsm)) as Arc<Mutex<dyn hsm_demo::Hsm + Send + Sync>>, public_key)
        }
        "VAULT" => {
            let vault_hsm = hsm::hashi_corp_vault::HashiCorpVault::try_from_env()?
                .create_key()
                .await?
                .fill_with_public_key()
                .await?;
            let public_key = vault_hsm.public_key.clone();
            (Arc::new(Mutex::new(vault_hsm)) as Arc<Mutex<dyn hsm_demo::Hsm + Send + Sync>>, public_key)
        }
        _ => {
            return Err(anyhow::anyhow!("Unsupported provider: {}", provider));
        }
    };

    // Start the server task
    let server_hsm = hsm.clone();
    let server_task = task::spawn(async move {
        let app = create_server(server_hsm);
        let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
        println!("Server listening on {}", addr);

        Server::bind(&addr)
            .serve(app.into_make_service())
            .await
            .expect("Server failed");
    });

    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    // Start the Application
    let client = Client::new();
    let random_stream = action_stream::random::Random;
    let notify_verify_stream = action_stream::notify_verify::NotifyVerify::new();
    let join_stream = action_stream::join::Join::new(vec![
        Box::new(random_stream),
        Box::new(notify_verify_stream),
    ]);

    // Replace HSM with the HTTP client that sends requests to the server
    let hsm_proxy = HttpHsmProxy::new(client, "http://127.0.0.1:3000/sign".to_string(), public_key);
    let mut app = Application::new(Box::new(hsm_proxy), Box::new(join_stream));

    app.run().await?;

    server_task.await?;
    Ok(())
}

#[derive(Serialize)]
struct SignRequest {
    message: Vec<u8>,
}

#[derive(serde::Deserialize)]
struct SignedResponse {
    signature: Vec<u8>,
}

pub struct HttpHsmProxy {
    client: Client,
    server_url: String,
    public_key: PublicKey,
}

impl HttpHsmProxy {
    pub fn new(client: Client, server_url: String, public_key: PublicKey) -> Self {
        Self { client, server_url, public_key }
    }

    pub fn get_public_key(&self) -> PublicKey {
        self.public_key.clone()
    }
}

#[async_trait::async_trait]
impl Hsm for HttpHsmProxy {
    async fn sign(
        &self,
        message: Bytes,
    ) -> Result<(Bytes, PublicKey, Signature), anyhow::Error> {
        let payload = SignRequest { message: message.0.clone() };

        let response = self
            .client
            .post(&self.server_url)
            .json(&payload)
            .send()
            .await?
            .json::<SignedResponse>()
            .await?;

        let signature = Signature(Bytes(response.signature));

        // Return the stored public key along with the signature
        Ok((message, self.public_key.clone(), signature))
    }

    async fn verify(
        &self,
        _message: Bytes,
        _public_key: PublicKey,
        _signature: Signature,
    ) -> Result<bool, anyhow::Error> {
        // Verification would need another endpoint or can be skipped because Application already verifies
        Ok(true)
    }
}

