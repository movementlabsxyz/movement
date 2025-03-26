use crate::error::DaSequencerError;
use crate::storage::Storage;
use movement_da_sequencer_config::DaSequencerConfig;

mod batch;
mod block;
mod celestia;
mod error;
mod server;
mod storage;

/// Run Da sequencing loop.
/// This function only return in case of error that indicate a crash of the node.
pub fn run(config: DaSequencerConfig) -> std::result::Result<(), DaSequencerError> {
	let path = "./";
	let store = Storage::try_new(path)?;
	Ok(())
}
