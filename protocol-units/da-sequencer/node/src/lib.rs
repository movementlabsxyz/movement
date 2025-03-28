use crate::error::DaSequencerError;
use crate::server::GrpcRequests;
use movement_da_sequencer_config::DaSequencerConfig;
use tokio::sync::mpsc;

pub mod batch;
mod block;
mod celestia;
pub mod error;
mod server;
mod storage;

/// Run Da sequencing loop.
/// This function only return in case of error that indicate a crash of the node.
pub async fn run(
	config: DaSequencerConfig,
	mut request_rx: mpsc::Receiver<GrpcRequests>,
) -> std::result::Result<(), DaSequencerError> {
	let mut produce_block_interval = tokio::time::interval(tokio::time::Duration::from_millis(500)); //todo put interval value in the config.
	loop {
		tokio::select! {
			Some(grpc_request) = request_rx.recv() => {
				match grpc_request {
					GrpcRequests::StartBlockStream { callback } => todo!(),
					GrpcRequests::GetBlockHeight { block_height, callback } => todo!(),
					GrpcRequests::WriteBatch(batch) => {
						//send batch to the storage.
					},

				}
			}
			_ = produce_block_interval.tick() => {
				//produce one block

				//propagate the new block.
			}
		}
	}
}
