use crate::error::DaSequencerError;
use crate::whitelist::Whitelist;
use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};
use aptos_types::transaction::SignedTransaction as AptosTransaction;
use bcs;
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
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

	// Validate all batch Txs signatures.
	// Return an error if any signature is invalid.
	// If any Tx is wrong, the whole batch is rejected.
	fn validate_txs(&self) -> Result<(), DaSequencerError> {
		self.0.iter().try_for_each(|tx| {
			//Validate batch Tx signature
			let aptos_transaction: AptosTransaction = bcs::from_bytes(&tx.data())
				.map_err(|_| DaSequencerError::DeserializationFailure)?;

			aptos_transaction
				.verify_signature()
				.map_err(|_| DaSequencerError::InvalidSignature)
		})
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
	whitelist: &Whitelist,
) -> Result<DaBatch<FullNodeTxs>, DaSequencerError> {
	if !new_batch.signer.verify(&new_batch.data().data, &new_batch.signature).is_ok() {
		return Err(DaSequencerError::InvalidSignature);
	}
	if !whitelist.contains(&new_batch.signer) {
		return Err(DaSequencerError::UnauthorizedSigner);
	}

	let data = bcs::from_bytes::<FullNodeTxs>(&new_batch.data.data)
		.map_err(|_| DaSequencerError::DeserializationFailure)?;

	//Validate batch Txs
	// If any Tx is wrong, the whole batch is rejected.
	// Authenticated batch sender should never send bad Tx.
	data.validate_txs()?;

	Ok(DaBatch {
		data,
		signature: new_batch.signature,
		signer: new_batch.signer,
		timestamp: new_batch.timestamp,
	})
}
