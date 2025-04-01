use crate::error::DaSequencerError;
use crate::whitelist::Whitelist;
use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};
use bcs;
use ed25519_dalek::{Signature, Signer, Verifier, VerifyingKey};
use movement_da_sequencer_config::DaSequencerConfig;
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

#[derive(Debug, Clone)]
pub struct DaBatch<D> {
	pub data: D,
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

pub fn serialize_full_node_batch(
	signer: VerifyingKey,
	signature: Signature,
	mut data: Vec<u8>,
) -> Vec<u8> {
	let mut serialized: Vec<u8> = Vec::with_capacity(64 + 32 + data.len());
	serialized.extend_from_slice(&signer.to_bytes());
	serialized.extend_from_slice(&signature.to_bytes());
	serialized.append(&mut data);
	serialized
}

pub fn deserialize_full_node_batch(
	data: Vec<u8>,
) -> Result<(VerifyingKey, Signature, Vec<u8>), DaSequencerError> {
	let (pubkey_deserialized, rest) = data.split_at(32);
	let (sign_deserialized, vec_deserialized) = rest.split_at(64);

	let pub_key_bytes: [u8; 32] = pubkey_deserialized.try_into()?;
	let signature_bytes: [u8; 64] = sign_deserialized.try_into()?;

	let public_key = VerifyingKey::try_from(pub_key_bytes.as_slice())
		.map_err(|_| DaSequencerError::DeserializationFailure)?;
	let signature = Signature::try_from(signature_bytes.as_slice())
		.map_err(|_| DaSequencerError::DeserializationFailure)?;

	let data: Vec<u8> = vec_deserialized.to_vec();
	Ok((public_key, signature, data))
}

pub fn validate_batch(
	new_batch: DaBatch<RawData>,
) -> Result<DaBatch<FullNodeTxs>, DaSequencerError> {
	let whitelist = Whitelist::get();

	verify_batch_signature(&new_batch.data.data, &new_batch.signature, &new_batch.signer)?;

	if !whitelist.contains(&new_batch.signer) {
		return Err(DaSequencerError::InvalidSigner);
	}

	let txs: FullNodeTxs = bcs::from_bytes(&new_batch.data.data)
		.map_err(|_| DaSequencerError::DeserializationFailure)?;

	Ok(DaBatch {
		data: txs,
		signature: new_batch.signature,
		signer: new_batch.signer,
		timestamp: new_batch.timestamp,
	})
}

pub fn verify_batch_signature(
	batch_data: &[u8],
	signature: &Signature,
	public_key: &VerifyingKey,
) -> Result<(), DaSequencerError> {
	public_key
		.verify(batch_data, signature)
		.map_err(|_| DaSequencerError::InvalidSignature)
}

#[cfg(test)]
impl<D> DaBatch<D>
where
	D: Serialize + aptos_crypto::hash::CryptoHash,
{
	pub fn test_only_new(data: D) -> Self {
		let config = DaSequencerConfig::default();
		let private_key = config.signing_key;
		let public_key = private_key.verifying_key();

		let serialized = bcs::to_bytes(&data).unwrap();
		let signature = private_key.sign(&serialized);
		let timestamp = chrono::Utc::now().timestamp_micros() as u64;

		Self { data, signature, signer: public_key, timestamp }
	}
}
