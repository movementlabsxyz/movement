use axum::Server;
use hsm_demo::{hsm, Bytes};
use reqwest::Client;
use serde::Serialize;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::task;

use hsm_demo::server::create_server;

#[derive(Serialize)]
struct SignRequest {
    message: Vec<u8>,
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let aws_kms_hsm = hsm::aws_kms::AwsKms::try_from_env()
        .await?
        .create_key()
        .await?
        .fill_with_public_key()
        .await?;

    let shared_hsm = Arc::new(Mutex::new(aws_kms_hsm));

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

    let client = Client::new();
    let messages = vec![
        Bytes(b"Hello, AWS KMS!".to_vec()),
        Bytes(b"Signing this message.".to_vec()),
        Bytes(b"Test message 12345.".to_vec()),
    ];

    for message in messages {
        let payload = SignRequest { message: message.0 };

        let response = client
            .post("http://127.0.0.1:3000/sign")
            .json(&payload)
            .send()
            .await
            .expect("Failed to send request");

        let signed_response: SignedResponse = response
            .json()
            .await
            .expect("Failed to parse response");

        println!(
            "Signed Message: {:?}, Signature: {:?}",
            payload.message, signed_response.signature
        );
    }

    server_task.await?;
    Ok(())
}

#[derive(serde::Deserialize)]
struct SignedResponse {
    signature: Vec<u8>,
}
