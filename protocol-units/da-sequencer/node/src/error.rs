/// Errors thrown by Da Sequencer.
#[derive(Debug, thiserror::Error)]
pub enum DaSequencerError {
	#[error("Error during storage access: {0}")]
	StorageAccess(String),
	#[error("Error during bootstrapping the external DA: {0}")]
	ExternalDaBootstrap(String),
	#[error("Error during requesting a block: {0}")]
	BlockRetrieval(String),
}
