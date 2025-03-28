/// Errors thrown by Da Sequencer.
#[derive(Debug, thiserror::Error)]
pub enum DaSequencerError {
	#[error("Error during storage access: {0}")]
	StorageAccess(String),
	#[error("Storage format error: {0}")]
	StorageFormat(String),
	#[error("Size Exceeds max: {0}")]
	SizeExceedsMax(usize),
	#[error("RocksDB operation failed: {0}")]
	RocksDbError(String),
	#[error("Deserialization error: {0}")]
	Deserialization(String),
	#[error("Invalid path error: {0}")]
	InvalidPath(String),
	#[error("Error during storage access: {0}")]
	BatchSerializationError(#[from] std::array::TryFromSliceError),
	#[error("Key or signature are badly formated: {0}")]
	BadKeyOrSign(#[from] aptos_sdk::crypto::CryptoMaterialError),
}
