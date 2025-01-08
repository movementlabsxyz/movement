use axum::{
    routing::post,
    extract::State,
    Json, Router,
    http::StatusCode,
};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::{Bytes, Hsm}; 

pub fn create_server(hsm: Arc<Mutex<dyn Hsm + Send + Sync>>) -> Router {
    Router::new()
        .route("/sign", post(sign_handler))
        .with_state(hsm)
}

async fn sign_handler(
    State(hsm): State<Arc<Mutex<dyn Hsm + Send + Sync>>>,
    Json(payload): Json<SignRequest>,
) -> Result<Json<SignedResponse>, StatusCode> {
    let message_bytes = Bytes(payload.message);

    let (_message, _public_key, signature) = hsm
        .lock()
        .await
        .sign(message_bytes)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(SignedResponse {
        signature: signature.0 .0, 
    }))
}

#[derive(serde::Deserialize)]
pub struct SignRequest {
    pub message: Vec<u8>,
}

#[derive(serde::Serialize)]
pub struct SignedResponse {
    pub signature: Vec<u8>,
}
