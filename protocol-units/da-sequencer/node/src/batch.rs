use crate::error::DaSequencerError;
use aptos_crypto::ed25519::{Ed25519PublicKey, Ed25519Signature};
use core::convert::TryFrom;

#[derive(Debug)]
pub struct RawData {
	pub data: Vec<u8>,
}

#[derive(Debug)]
pub struct FullnodeTx {}

#[derive(Debug)]
pub struct DaBatch<Data> {
	pub data: Data,
	pub signature: Ed25519Signature,
	pub signer: Ed25519PublicKey,
	pub timestamp: u64,
}

impl DaBatch<RawData> {
	pub fn now(signer: Ed25519PublicKey, signature: Ed25519Signature, data: Vec<u8>) -> Self {
		let timestamp = chrono::Utc::now().timestamp_micros() as u64;
		DaBatch { data: RawData { data }, signature, signer, timestamp }
	}
}

/// Batch write blobs.
pub fn validate_batch(
	new_batch: DaBatch<RawData>,
) -> std::result::Result<DaBatch<FullnodeTx>, DaSequencerError> {
	todo!()
}

pub fn serialize_full_node_batch(
	signer: Ed25519PublicKey,
	signature: Ed25519Signature,
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
) -> std::result::Result<(Ed25519PublicKey, Ed25519Signature, Vec<u8>), DaSequencerError> {
	let (pubkey_deserialized, rest) = data.split_at(32);
	let (sign_deserialized, vec_deserialized) = rest.split_at(64);

	// Convert the slices back into arrays
	let pub_key_bytes: [u8; 32] = pubkey_deserialized.try_into()?;
	let signature_bytes: [u8; 64] = sign_deserialized.try_into()?;

	let public_key = Ed25519PublicKey::try_from(pub_key_bytes.as_slice())?;
	let signature = Ed25519Signature::try_from(signature_bytes.as_slice())?;

	let data: Vec<u8> = vec_deserialized.to_vec();
	Ok((public_key, signature, data))
}
