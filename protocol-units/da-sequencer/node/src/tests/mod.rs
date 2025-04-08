use crate::batch::DaBatch;
use crate::tests::whitelist::make_test_whitelist;
use aptos_crypto::hash::CryptoHash;
use ed25519_dalek::Signer;
use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;
use rand::RngCore;
use serde::Serialize;

pub mod client;
pub mod mock;
pub mod whitelist;

impl<D> DaBatch<D>
where
	D: Serialize + CryptoHash,
{
	/// Creates a test-only `DaBatch` with a real signature over the given data.
	/// Only usable in tests.
	pub fn test_only_new(data: D) -> Self
	where
		D: Serialize,
	{
		let private_key = generate_signing_key();
		let public_key = private_key.verifying_key();

		let serialized = bcs::to_bytes(&data).unwrap(); // only fails if serialization is broken
		let signature = private_key.sign(&serialized);
		let timestamp = chrono::Utc::now().timestamp_micros() as u64;

		Self { data, signature, signer: public_key, timestamp }
	}
}

pub fn generate_signing_key() -> SigningKey {
	let mut bytes = [0u8; 32];
	OsRng.fill_bytes(&mut bytes);
	let signing_key = SigningKey::from_bytes(&bytes);
	signing_key
}
