use axum::{extract::State, http::StatusCode, routing::post, Json, Router};
use movement_signer::cryptography::ToBytes;
use movement_signer::{cryptography::Curve, Signer, Signing};
use std::sync::Arc;
use tokio::sync::Mutex;

pub fn create_server<O, C>(hsm: Arc<Mutex<Signer<O, C>>>) -> Router
where
	O: Signing<C> + Send + Sync + 'static,
	C: Curve + Send + Sync + 'static,
{
	Router::new().route("/sign", post(sign_handler)).with_state(hsm)
}

async fn sign_handler<O, C>(
	State(hsm): State<Arc<Mutex<Signer<O, C>>>>,
	Json(payload): Json<SignRequest>,
) -> Result<Json<SignedResponse>, StatusCode>
where
	O: Signing<C>,
	C: Curve,
{
	println!("Received payload: {:?}", &payload);

	let message_bytes = payload.message.as_slice();

	println!(
		"Preparing to sign message. Message bytes: {:?}",
		message_bytes
	);

	let signature = hsm
		.lock()
		.await
		.sign(message_bytes)
		.await
		.map_err(|e| {
			println!("Error signing message: {:?}", e);
			StatusCode::INTERNAL_SERVER_ERROR
		})?;

	println!("Generated signature: {:?}", signature);

	Ok(Json(SignedResponse { signature: signature.to_bytes() }))
}

#[derive(Debug, serde::Deserialize)]
pub struct SignRequest {
	pub message: Vec<u8>,
}

#[derive(serde::Serialize)]
pub struct SignedResponse {
	pub signature: Vec<u8>,
}
