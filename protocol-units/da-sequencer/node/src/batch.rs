use crate::error::DaSequencerError;
use aptos_crypto::ed25519::{Ed25519PublicKey, Ed25519Signature};
use aptos_crypto::hash::CryptoHash;
use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};
use movement_types::transaction::Transaction;
use serde::{ser::SerializeMap, Deserialize, Serialize};
use std::ops::Deref;

#[derive(Debug)]
pub struct RawData {
	pub bytes: Vec<u8>,
}

///We want to distinguish here between FullNode transactions and DA Transactions
#[derive(CryptoHasher, BCSCryptoHash, Deserialize, Serialize)]
pub struct FullNodeTxs(Vec<Transaction>);

impl FullNodeTxs {
	pub fn new(txs: Vec<Transaction>) -> Self {
		FullNodeTxs(txs)
	}
}

impl Deref for FullNodeTxs {
	type Target = Vec<Transaction>;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

#[derive(Debug)]
pub struct DaBatch<T> {
	data: T,
	signature: Ed25519Signature,
	signer: Ed25519PublicKey,
}

impl<T> DaBatch<T> {
	pub fn data(&self) -> &T {
		&self.data
	}
}

#[cfg(test)]
impl<T> DaBatch<T>
where
	T: Serialize + CryptoHash,
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
) -> std::result::Result<DaBatch<FullNodeTxs>, DaSequencerError> {
	todo!()
}
