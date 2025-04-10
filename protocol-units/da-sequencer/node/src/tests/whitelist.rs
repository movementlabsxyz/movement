use crate::batch::*;
use crate::tests::generate_signing_key;
use crate::whitelist::Whitelist;
use ed25519_dalek::Signer;
use ed25519_dalek::VerifyingKey;
use movement_da_sequencer_client::deserialize_full_node_batch;
use movement_da_sequencer_client::serialize_full_node_batch;
use movement_signer::cryptography::ed25519::Signature as SigningSignature;
use movement_types::transaction::Transaction;
use std::collections::HashSet;
use std::sync::{Arc, RwLock};

pub fn make_test_whitelist(keys: Vec<VerifyingKey>) -> Whitelist {
	Whitelist::from_keys(keys)
}

#[cfg(test)]
impl Whitelist {
	pub fn from_keys(keys: Vec<VerifyingKey>) -> Self {
		let set = keys.into_iter().collect::<HashSet<_>>();
		Self { inner: Arc::new(RwLock::new(set)) }
	}

	pub fn set_keys(&mut self, keys: Vec<VerifyingKey>) {
		let new_set: HashSet<_> = keys.into_iter().collect();
		*self.inner.write().unwrap() = new_set;
	}

	pub fn clear(&mut self) {
		self.inner.write().unwrap().clear();
	}

	pub fn insert(&mut self, key: VerifyingKey) {
		self.inner.write().unwrap().insert(key);
	}
}

#[tokio::test]
async fn test_sign_and_validate_batch_passes_with_whitelisted_signer() {
	let signing_key = generate_signing_key();
	let verifying_key = signing_key.verifying_key();
	let whitelist = make_test_whitelist(vec![verifying_key]);

	let txs = FullNodeTxs(vec![
		Transaction::new(b"hello".to_vec(), 0, 1),
		Transaction::new(b"world".to_vec(), 0, 2),
	]);

	let batch_bytes = bcs::to_bytes(&txs).expect("Serialization failed");
	let signature = signing_key.sign(&batch_bytes);
	let signature = SigningSignature::try_from(&signature.to_bytes()[..]).unwrap();
	let serialized =
		serialize_full_node_batch(verifying_key, signature.clone(), batch_bytes.clone());

	let (deserialized_key, deserialized_sig, deserialized_data) =
		deserialize_full_node_batch(serialized).expect("Deserialization failed");

	let raw_batch = DaBatch {
		data: RawData { data: deserialized_data },
		signature: deserialized_sig,
		signer: deserialized_key,
		timestamp: chrono::Utc::now().timestamp_micros() as u64,
	};

	let validated = validate_batch(raw_batch, &whitelist).expect("Batch should validate");
	assert_eq!(validated.data.0, txs.0);
}

#[tokio::test]
async fn test_sign_and_validate_batch_fails_with_non_whitelisted_signer() {
	// Create a key that *will* be whitelisted (but not used to sign)
	let whitelist_signing_key = generate_signing_key();
	let whitelisted_key = whitelist_signing_key.verifying_key();

	// Whitelist only the whitelisted_key, not the actual signer
	let whitelist = make_test_whitelist(vec![whitelisted_key]);

	// Create a *different* key that will actually be used to sign
	let signing_key = generate_signing_key();
	let verifying_key = signing_key.verifying_key();

	let txs = FullNodeTxs(vec![
		Transaction::new(b"hello".to_vec(), 0, 1),
		Transaction::new(b"world".to_vec(), 0, 2),
	]);

	let batch_bytes = bcs::to_bytes(&txs).expect("Serialization failed");
	let signature = signing_key.sign(&batch_bytes);
	let signature = SigningSignature::try_from(&signature.to_bytes()[..]).unwrap();

	let serialized =
		serialize_full_node_batch(verifying_key, signature.clone(), batch_bytes.clone());

	let (deserialized_key, deserialized_sig, deserialized_data) =
		deserialize_full_node_batch(serialized).expect("Deserialization failed");

	let raw_batch = DaBatch {
		data: RawData { data: deserialized_data },
		signature: deserialized_sig,
		signer: deserialized_key,
		timestamp: chrono::Utc::now().timestamp_micros() as u64,
	};

	let result = validate_batch(raw_batch, &whitelist);
	assert!(matches!(result, Err(crate::error::DaSequencerError::UnauthorizedSigner)));
}
