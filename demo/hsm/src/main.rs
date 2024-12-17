use axum::Server;
use hsm_demo::{hsm, Bytes};
use reqwest::Client;
use serde::Serialize;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::task;

use hsm_demo::{action_stream, Application};
use hsm_demo::server::create_server;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // Initialize AWS KMS HSM
    let aws_kms_hsm = hsm::aws_kms::AwsKms::try_from_env()
        .await?
        .create_key()
        .await?
        .fill_with_public_key()
        .await?;

    let shared_hsm = Arc::new(Mutex::new(aws_kms_hsm));

    // Start the server task
    let server_hsm = shared_hsm.clone();
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
    let hsm_proxy = HttpHsmProxy::new(client, "http://127.0.0.1:3000/sign".to_string());
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

struct HttpHsmProxy {
    client: Client,
    server_url: String,
}

impl HttpHsmProxy {
    pub fn new(client: Client, server_url: String) -> Self {
        Self { client, server_url }
    }
}

#[async_trait::async_trait]
impl hsm_demo::Hsm for HttpHsmProxy {
    async fn sign(
        &self,
        message: Bytes,
    ) -> Result<(Bytes, hsm_demo::PublicKey, hsm_demo::Signature), anyhow::Error> {
        let payload = SignRequest { message: message.0.clone() };

        let response = self
            .client
            .post(&self.server_url)
            .json(&payload)
            .send()
            .await?
            .json::<SignedResponse>()
            .await?;

        let signature = hsm_demo::Signature(Bytes(response.signature));
        let public_key = hsm_demo::PublicKey(Bytes(vec![])); // Public key is not returned here

        Ok((message, public_key, signature))
    }

    async fn verify(
        &self,
        _message: Bytes,
        _public_key: hsm_demo::PublicKey,
        _signature: hsm_demo::Signature,
    ) -> Result<bool, anyhow::Error> {
        // Verification would need another endpoint or can be skipped for now
        Ok(true)
    }
}
