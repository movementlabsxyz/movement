/// Errors thrown by Da Sequencer.
#[derive(Debug, thiserror::Error)]
pub enum DaSequencerError {
	#[error("Error during storage access: {0}")]
	StorageAccess(String),
	#[error("Generic error: {0}")]
	Generic(String),
}
