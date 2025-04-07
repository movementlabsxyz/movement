use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;
use rand::RngCore;

pub mod client;
pub mod mock;

pub fn generate_signing_key() -> SigningKey {
	let mut bytes = [0u8; 32];
	OsRng.fill_bytes(&mut bytes);
	let signing_key = SigningKey::from_bytes(&bytes);
	signing_key
}
