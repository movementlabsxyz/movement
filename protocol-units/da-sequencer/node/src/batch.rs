use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};
use crate::error::DaSequencerError;
use bcs;
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use movement_types::transaction::Transaction;
use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub struct RawData {
        pub bytes: Vec<u8>,
}

#[derive(CryptoHasher, BCSCryptoHash, Deserialize, Serialize, PartialEq, Debug)]
pub struct FullNodeTxs(pub Vec<Transaction>);

#[derive(Debug)]
pub struct DaBatch<T> {
        pub data: T,
        pub signature: Signature,
        pub signer: VerifyingKey,
}

pub fn validate_batch(
        new_batch: DaBatch<RawData>,
) -> Result<DaBatch<FullNodeTxs>, DaSequencerError> {
        verify_batch_signature(&new_batch.data.bytes, &new_batch.signature, &new_batch.signer)?;

        let txs: FullNodeTxs = bcs::from_bytes(&new_batch.data.bytes)
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
        use movement_da_sequencer_client::sign_batch;
        use movement_da_sequencer_config::DaSequencerConfig;
        use tracing_subscriber;

        #[test]
        fn test_sign_and_validate_batch() {
                let _ = tracing_subscriber::fmt()
                        .with_max_level(tracing::Level::INFO)
                        .with_test_writer()
                        .try_init();

                let config = DaSequencerConfig::default();
                let signing_key = config.signing_key;
                let verifying_key = signing_key.verifying_key();

                let txs = FullNodeTxs(vec![
                        Transaction::new(b"hello".to_vec(), 0, 1),
                        Transaction::new(b"world".to_vec(), 0, 2),
                    ]);
                    
                    let batch_bytes = bcs::to_bytes(&txs).expect("Serialization failed");
                    
                    let signature = sign_batch(&batch_bytes, &signing_key);
                    
                    let raw_batch = DaBatch {
                        data: RawData { bytes: batch_bytes.clone() },
                        signature,
                        signer: verifying_key,
                    };
                    
                    let validated = validate_batch(raw_batch).expect("Batch should validate");
                    
                    // Compare the inner vecs directly
                    assert_eq!(validated.data.0.len(), 2);
                    assert_eq!(validated.data.0, txs.0);
        }
}
