use crate::error::DaSequencerError;
use ed25519_dalek::{Signature, VerifyingKey, Verifier};
use serde::{Serialize, Deserialize};
use bcs;
use movement_da_sequencer_client::{generate_keypair, sign_batch};

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

        Ok(DaBatch {
                data: txs,
                signature: new_batch.signature,
                signer: new_batch.signer,
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
mod tests {
        use super::*;

        #[test]
        fn test_sign_and_validate_batch() {
                // Generate a test keypair
                let (signing_key, verifying_key) = generate_keypair();

                // Create a sample list of transactions
                let txs = vec![
                        FullnodeTx { id: 1, data: "hello".to_string() },
                        FullnodeTx { id: 2, data: "world".to_string() },
                ];

                // Serialize transactions into bytes
                let tx_bytes = bcs::to_bytes(&txs).expect("Serialization failed");

                // Sign the batch
                let signature = sign_batch(&tx_bytes, &signing_key);

                // Construct a raw DaBatch with just the bytes
                let raw_batch = DaBatch {
                        data: RawData { bytes: tx_bytes.clone() },
                        signature,
                        signer: verifying_key,
                };

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

