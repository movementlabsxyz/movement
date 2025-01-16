use axum::{
        http::StatusCode, routing::{get, post}, Extension, Json, Router
};
use movement_signer::cryptography::ToBytes;
use movement_signer::{cryptography::Curve, Signer, Signing};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;
use ed25519_dalek::{VerifyingKey, Signature, Verifier};


pub struct AppState {
        pub public_key: Mutex<Option<Vec<u8>>>, 
}

impl AppState {
        pub fn new() -> Self {
                Self {
                        public_key: Mutex::new(None), 
                }
        }
}

pub fn create_server<O, C>(
        hsm: Arc<Mutex<Signer<O, C>>>,
        app_state: Arc<AppState>,
) -> Router
where
        O: Signing<C> + Send + Sync + 'static,
        C: Curve + Send + Sync + 'static,
{
        Router::new()
                .route("/sign", post(sign_handler::<O, C>))
                .route("/verify", post(verify_handler))
                .route("/health", get(health_handler))
                .route("/public_key/get", get(get_public_key)) 
                .route("/public_key/set", post(set_public_key)) 
                .layer(Extension(hsm))                         
                .layer(Extension(app_state))                   
}

    

// Health check endpoint
async fn health_handler() -> &'static str {
        "OK"
}

// /sign endpoint for signing a message
async fn sign_handler<O, C>(
        Extension(hsm): Extension<Arc<Mutex<Signer<O, C>>>>,
        Extension(app_state): Extension<Arc<AppState>>,
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

        // Perform the signing
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

        // Retrieve the public key
        let public_key = hsm
                .lock()
                .await
                .public_key()
                .await
                .map_err(|e| {
                        println!("Error retrieving public key: {:?}", e);
                        StatusCode::INTERNAL_SERVER_ERROR
                })?;

        println!("Retrieved public key: {:?}", public_key.to_bytes());

        // Return both the signature and public key
        Ok(Json(SignedResponse {
                signature: signature.to_bytes(),
                public_key: public_key.to_bytes(),
        }))
}
    

// Request and response types for /sign
#[derive(Debug, Deserialize)]
pub struct SignRequest {
        pub message: Vec<u8>,
}

#[derive(Serialize)]
pub struct SignedResponse {
        pub signature: Vec<u8>,
	pub public_key: Vec<u8>,
}

// /verify endpoint for verifying a signature
#[derive(Debug, Deserialize)]
pub struct VerifyRequest {
        pub message: Vec<u8>,
        pub signature: Vec<u8>,
        pub public_key: Vec<u8>,
        pub algorithm: String,
} 

#[derive(Debug, Serialize)]
pub struct VerifyResponse {
        pub valid: bool,
}

async fn verify_handler(
	Json(payload): Json<VerifyRequest>,
) -> Result<Json<VerifyResponse>, StatusCode> {
	match payload.algorithm.as_str() {
	"ed25519" => verify_ed25519(&payload).await,
	"ecdsa" => verify_ecdsa(&payload).await, 
		_ => {
			println!("Unsupported algorithm: {}", payload.algorithm);
			Err(StatusCode::BAD_REQUEST)
		}
	}
}

async fn verify_ecdsa(payload: &VerifyRequest) -> Result<Json<VerifyResponse>, StatusCode> {
	use k256::ecdsa::{signature::Verifier as _, Signature, VerifyingKey};

	// Convert the public key from the payload
	let public_key = VerifyingKey::from_sec1_bytes(&payload.public_key).map_err(|_| {
		println!("Invalid public key format for ECDSA");
		StatusCode::BAD_REQUEST
	})?;

	// Convert the signature from the payload
	let signature = Signature::from_der(&payload.signature).map_err(|_| {
		println!("Invalid signature format for ECDSA");
		StatusCode::BAD_REQUEST
	})?;

	// Verify the signature
	let valid = public_key.verify(&payload.message, &signature).is_ok();

	Ok(Json(VerifyResponse { valid }))
}


async fn verify_ed25519(payload: &VerifyRequest) -> Result<Json<VerifyResponse>, StatusCode> {
        // Convert the public key
	let public_key_bytes: &[u8; 32] = payload
        .public_key
        .as_slice()
        .try_into()
        .map_err(|_| {
                println!("Invalid public key length for ed25519");
                StatusCode::BAD_REQUEST
        })?;

	let verifying_key = VerifyingKey::from_bytes(public_key_bytes).map_err(|_| {
		println!("Invalid public key format for ed25519");
		StatusCode::BAD_REQUEST
	})?;

        // Convert the signature
        let signature_bytes: &[u8; 64] = payload
                .signature
                .as_slice()
                .try_into()
                .map_err(|_| {
                        println!("Invalid signature length for ed25519");
                        StatusCode::BAD_REQUEST
                })?;

	//use std::convert::TryFrom;

	let signature = Signature::try_from(signature_bytes).map_err(|_| {
		println!("Invalid signature format for ed25519");
		StatusCode::BAD_REQUEST
	})?;

        // Verify the signature
        let valid = verifying_key.verify(&payload.message, &signature).is_ok();

        Ok(Json(VerifyResponse { valid }))
}

pub async fn get_public_key(
        Extension(app_state): Extension<Arc<AppState>>,
) -> Result<Json<Vec<u8>>, StatusCode> {
        let public_key = app_state.public_key.lock().await;

        if let Some(key) = &*public_key {
                Ok(Json(key.clone()))
        } else {
                Err(StatusCode::NOT_FOUND)
        }
}



#[derive(Deserialize)]
pub struct SetPublicKeyRequest {
        pub public_key: Vec<u8>,
}

pub async fn set_public_key(
        Extension(app_state): Extension<Arc<AppState>>,
        Json(payload): Json<SetPublicKeyRequest>,
) -> StatusCode {
        // Lock the public key mutex and set the new public key
        let mut public_key = app_state.public_key.lock().await;
        *public_key = Some(payload.public_key.clone());

        println!("Public key has been set to: {:?}", payload.public_key);
        StatusCode::OK
}

