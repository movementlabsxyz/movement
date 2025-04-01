use crate::error::DaSequencerError;
use crate::whitelist::{Whitelist, INSTANCE};
use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};
use bcs;
use ed25519_dalek::{Signer as _, SigningKey, Signature, Verifier, VerifyingKey};
use movement_types::transaction::Transaction;
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use std::ops::Deref;
use serial_test::serial;

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
) -> std::result::Result<(VerifyingKey, Signature, Vec<u8>), DaSequencerError> {
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

        if !whitelist.contains(&new_batch.signer) {
                return Err(DaSequencerError::InvalidSigner);
        }

        verify_batch_signature(&new_batch.data.data, &new_batch.signature, &new_batch.signer)?;

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
mod tests {
        use super::*;
        use aptos_crypto::hash::CryptoHash;
        use ed25519_dalek::Signer;
        use rand::rngs::OsRng;
        use rand_core::{CryptoRng, RngCore};

        use movement_da_sequencer_client::sign_batch;
        use movement_da_sequencer_config::DaSequencerConfig;
        use tracing_subscriber;

        impl<D> DaBatch<D>
        where
                D: Serialize + CryptoHash,
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

        #[test]
        #[serial]
        fn test_sign_and_validate_batch_passes_with_whitelisted_signer() {
                let _ = tracing_subscriber::fmt()
                        .with_max_level(tracing::Level::INFO)
                        .with_test_writer()
                        .try_init();
        
                let config = DaSequencerConfig::default();
                let signing_key = config.signing_key;
                let verifying_key = signing_key.verifying_key();
        
                // Add signer to the whitelist
                let _ = INSTANCE.set(Whitelist::from_keys(vec![verifying_key]));
        
                let txs = FullNodeTxs(vec![
                        Transaction::new(b"hello".to_vec(), 0, 1),
                        Transaction::new(b"world".to_vec(), 0, 2),
                ]);
        
                let batch_bytes = bcs::to_bytes(&txs).expect("Serialization failed");
                let signature = sign_batch(&batch_bytes, &signing_key);
        
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
        
                let validated = validate_batch(raw_batch).expect("Batch should validate");
                assert_eq!(validated.data.0, txs.0);
        }
        
        #[test]
        #[serial]
        fn test_sign_and_validate_batch_fails_with_non_whitelisted_signer() {
                let _ = tracing_subscriber::fmt()
                        .with_max_level(tracing::Level::INFO)
                        .with_test_writer()
                        .try_init();
        
                // Not adding this key to the whitelist
                let config = DaSequencerConfig::default();
                let signing_key = config.signing_key;
                let verifying_key = signing_key.verifying_key();
        
                let txs = FullNodeTxs(vec![
                        Transaction::new(b"hello".to_vec(), 0, 1),
                        Transaction::new(b"world".to_vec(), 0, 2),
                ]);
        
                let batch_bytes = bcs::to_bytes(&txs).expect("Serialization failed");
                let signature = sign_batch(&batch_bytes, &signing_key);
        
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
        
                let result = validate_batch(raw_batch);
                assert!(matches!(result, Err(DaSequencerError::InvalidSigner)));
        }    
}
