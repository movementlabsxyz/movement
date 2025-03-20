use crate::error::DaSequencerError;
use aptos_crypto::ed25519::{Ed25519PublicKey, Ed25519Signature};

#[derive(Debug)]
pub struct RawData {
	pub bytes: Vec<u8>,
}

#[derive(Debug)]
pub struct FullnodeTx {}

#[derive(Debug)]
pub struct DaBatch<Data> {
	data: Data,
	signature: Ed25519Signature,
	signer: Ed25519PublicKey,
}

/// Batch write blobs.
fn validate_batch(
	new_batch: DaBatch<RawData>,
) -> std::result::Result<DaBatch<FullnodeTx>, DaSequencerError> {
	todo!()
}
