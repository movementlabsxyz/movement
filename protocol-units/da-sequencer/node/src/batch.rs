use crate::error::DaSequencerError;
use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};
use bcs;
use ed25519_dalek::{Signature, VerifyingKey};
use movement_types::transaction::Transaction;
use serde::{Deserialize, Serialize};
use std::ops::Deref;

#[derive(Debug, Clone)]
pub struct RawData {
	pub data: Vec<u8>,
}

#[derive(Deserialize, CryptoHasher, BCSCryptoHash, Serialize, PartialEq, Debug, Clone)]
pub struct FullNodeTxs(pub Vec<Transaction>);

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

impl<T> DaBatch<T> {
	pub fn data(&self) -> &T {
		&self.data
	}
}

#[derive(Debug, Clone)]
pub struct DaBatch<T> {
	pub data: T,
	pub signature: Signature,
	pub signer: VerifyingKey,
	pub timestamp: u64,
}

impl DaBatch<RawData> {
	pub fn now(signer: VerifyingKey, signature: Signature, data: Vec<u8>) -> Self {
		let timestamp = chrono::Utc::now().timestamp_micros() as u64;
		DaBatch { data: RawData { data }, signature, signer, timestamp }
	}
}

pub fn validate_batch(
	new_batch: DaBatch<RawData>,
) -> Result<DaBatch<FullNodeTxs>, DaSequencerError> {
	movement_da_sequencer_client::verify_batch_signature(
		&new_batch.data.data,
		&new_batch.signature,
		&new_batch.signer,
	)
	.map_err(|_| DaSequencerError::InvalidSignature)?;

	let txs: FullNodeTxs = bcs::from_bytes(&new_batch.data.data)
		.map_err(|_| DaSequencerError::DeserializationFailure)?;

	Ok(DaBatch {
		data: txs,
		signature: new_batch.signature,
		signer: new_batch.signer,
		timestamp: new_batch.timestamp,
	})
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::tests::generate_signing_key;
	use aptos_crypto::hash::CryptoHash;
	use ed25519_dalek::Signer;
	use movement_signer::cryptography::ed25519::Signature as SigningSignature;

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

	#[tokio::test]
	async fn test_sign_and_validate_batch() {
		let signing_key = generate_signing_key();
		let verifying_key = signing_key.verifying_key();

		// Create transactions and batch
		let txs = FullNodeTxs(vec![
			Transaction::new(b"hello".to_vec(), 0, 1),
			Transaction::new(b"world".to_vec(), 0, 2),
		]);

		let batch_bytes = bcs::to_bytes(&txs).expect("Serialization failed");

		let signature = signing_key.sign(&batch_bytes);
		let signature = SigningSignature::try_from(&signature.to_bytes()[..]).unwrap();

		// Serialize full node batch into raw bytes
		let serialized = movement_da_sequencer_client::serialize_full_node_batch(
			verifying_key,
			signature.clone(),
			batch_bytes.clone(),
		);

		// Deserialize it back
		let (deserialized_key, deserialized_sig, deserialized_data) =
			movement_da_sequencer_client::deserialize_full_node_batch(serialized)
				.expect("Deserialization failed");

		// Recreate the raw batch from deserialized data
		let raw_batch = DaBatch {
			data: RawData { data: deserialized_data },
			signature: deserialized_sig,
			signer: deserialized_key,
			timestamp: chrono::Utc::now().timestamp_micros() as u64,
		};

		// Validate the batch
		let validated = validate_batch(raw_batch).expect("Batch should validate");

		// Check it worked
		assert_eq!(validated.data.0.len(), 2);
		assert_eq!(validated.data.0, txs.0);
	}
}
