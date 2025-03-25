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
impl<T> DaBatch<T> {
	/// Test-only constructor to build a batch with dummy signature and signer.
	pub fn test_only_new(data: T) -> Self {
		Self { data, signature: Ed25519Signature::default(), signer: Ed25519PublicKey::default() }
	}
}

/// Batch write blobs.
fn validate_batch(
	new_batch: DaBatch<RawData>,
) -> std::result::Result<DaBatch<FullNodeTx>, DaSequencerError> {
	todo!()
}
