use crate::error::DaSequencerError;
use movement_da_sequencer_config::DaSequencerConfig;

mod batch;
mod block;
mod celestia;
mod error;
mod storage;

/// Run Da sequencing loop.
/// This function only return in case of error that indicate a crash of the node.
pub fn run(config: DaSequencerConfig) -> std::result::Result<(), DaSequencerError> {
	todo!()
}
