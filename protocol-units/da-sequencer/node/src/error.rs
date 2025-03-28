/// Errors thrown by Da Sequencer.
#[derive(Debug, thiserror::Error)]
pub enum DaSequencerError {
	#[error("Error during storage access: {0}")]
	StorageAccess(String),
	#[error("Error during batch serialization: {0}")]
	BatchSerializationError(#[from] std::array::TryFromSliceError),
	#[error("Key or signature are badly formated: {0}")]
	BadKeyOrSign(#[from] aptos_sdk::crypto::CryptoMaterialError),
        #[error("Failed to deserialize FullnodeTx batch")]
        DeserializationFailure,
        #[error("Signature was invalid")]
        InvalidSignature,
}
