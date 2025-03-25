use crate::error::DaSequencerError;
use aptos_crypto::ed25519::{Ed25519PublicKey, Ed25519Signature};

#[derive(Debug)]
pub struct RawData {
	pub bytes: Vec<u8>,
}

///We want to distinguish here between FullNode transactions and DA Transactions
pub type FullNodeTx = movement_types::transaction::Transaction;

#[derive(Debug)]
pub struct DaBatch<T> {
	pub data: T,
	signature: Ed25519Signature,
	signer: Ed25519PublicKey,
}

#[cfg(test)]
impl<T> DaBatch<T>
where
	T: serde::Serialize + aptos_crypto::hash::CryptoHash,
{
	/// Creates a test-only `DaBatch` with a real signature over the given data.
	/// Only usable in tests.
	pub fn test_only_new(data: T) -> Self {
		use aptos_crypto::ed25519::Ed25519PrivateKey;
		use aptos_crypto::{PrivateKey, SigningKey, Uniform};
		use rand::rngs::OsRng;

		let mut rng = OsRng;
		let private_key = Ed25519PrivateKey::generate(&mut rng);
		let public_key = private_key.public_key();

		// Sign the real data
		let signature = private_key.sign(&data).expect("Failed to sign test data");

		Self { data, signature, signer: public_key }
	}
}

/// Batch write blobs.
fn validate_batch(
	new_batch: DaBatch<RawData>,
) -> std::result::Result<DaBatch<FullNodeTx>, DaSequencerError> {
	todo!()
}
