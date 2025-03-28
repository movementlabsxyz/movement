use crate::error::DaSequencerError;
use bcs;
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub struct RawData {
	pub bytes: Vec<u8>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct FullnodeTx {
	pub id: u64,
	pub data: String,
}

#[derive(Debug)]
pub struct DaBatch<Data> {
	pub data: Data,
	pub signature: Signature,
	pub signer: VerifyingKey,
}

pub fn validate_batch(
	new_batch: DaBatch<RawData>,
) -> Result<DaBatch<Vec<FullnodeTx>>, DaSequencerError> {
	verify_batch_signature(&new_batch.data.bytes, &new_batch.signature, &new_batch.signer)?;

	let txs: Vec<FullnodeTx> = bcs::from_bytes(&new_batch.data.bytes)
		.map_err(|_| DaSequencerError::DeserializationFailure)?;

	Ok(DaBatch { data: txs, signature: new_batch.signature, signer: new_batch.signer })
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
mod tests {

	use movement_da_sequencer_client::sign_batch;
	use movement_da_sequencer_config::DaSequencerConfig;
	use tracing_subscriber;

	use super::*;

	#[test]
	fn test_sign_and_validate_batch() {
                let _ = tracing_subscriber::fmt()
                        .with_max_level(tracing::Level::INFO)
                        .with_test_writer()
                        .try_init(); 

		// Get the signing key from the config default
		let config = DaSequencerConfig::default();
		let signing_key = config.signing_key;
		let verifying_key = signing_key.verifying_key();

		// Create a sample list of transactions
		let txs = vec![
			FullnodeTx { id: 1, data: "hello".to_string() },
			FullnodeTx { id: 2, data: "world".to_string() },
		];

		// Serialize transaction batch into bytes
		let batch_bytes = bcs::to_bytes(&txs).expect("Serialization failed");

		// Sign the batch
		let signature = sign_batch(&batch_bytes, &signing_key);

		// Construct a raw DaBatch with just the bytes
		let raw_batch =
			DaBatch { data: RawData { bytes: batch_bytes.clone() }, signature, signer: verifying_key };

		// Validate the batch
		let validated = validate_batch(raw_batch).expect("Batch should validate");

		// Check it worked
		assert_eq!(validated.data.len(), 2);
		assert_eq!(
			validated.data,
			vec![
				FullnodeTx { id: 1, data: "hello".to_string() },
				FullnodeTx { id: 2, data: "world".to_string() },
			]
		);
	}
}
