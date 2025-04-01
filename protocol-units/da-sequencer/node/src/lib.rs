use crate::block::SequencerBlock;
use crate::celestia::DaSequencerExternalDa;
use crate::error::DaSequencerError;
use crate::server::GrpcRequests;
use crate::storage::DaSequencerStorage;
use futures::stream::FuturesUnordered;
use movement_da_sequencer_config::DaSequencerConfig;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio_stream::StreamExt;

pub mod batch;
mod block;
mod celestia;
pub mod error;
pub mod server;
mod storage;
#[cfg(test)]
mod tests;

/// Run Da sequencing loop.
/// This function only return in case of error that indicate a crash of the node.
pub async fn run<D, S>(
	config: DaSequencerConfig,
	mut request_rx: mpsc::Receiver<GrpcRequests>,
	storage: S,
	celestia: D,
) -> Result<(), DaSequencerError>
where
	D: DaSequencerExternalDa + Send + 'static,
	S: DaSequencerStorage + Send + 'static,
{
	let mut produce_block_interval = tokio::time::interval(tokio::time::Duration::from_millis(
		config.movement_da_sequencer_block_production_interval_millisec,
	)); //todo put interval value in the config.
	let mut spawn_result_futures = FuturesUnordered::new();
	let mut produce_block_jh = None;

	loop {
		tokio::select! {
			// Manage grpc request.
			Some(grpc_request) = request_rx.recv() => {
				match grpc_request {
					GrpcRequests::StartBlockStream { callback } => todo!(),
					GrpcRequests::GetBlockHeight { block_height, callback } => {
						let get_block_jh = tokio::task::spawn_blocking({
							let storage = storage.clone();
							move || {storage.get_block_at_height(block_height)}
						});
						tokio::spawn(async move {
							let result = get_block_jh.await;
							//manage result
							let to_send = match result {
								Err(err) => {
									tracing::error!("spawn_blocking task failed: {err}");
									None
								}
								Ok(Err(err)) => {
									tracing::error!("Storage get_block_at_height return an error: {err}");
									None

								}
								Ok(Ok(block)) => block,
							};

							let _ = callback.send(to_send);
						});
					},
					GrpcRequests::WriteBatch(batch) => {
						//send batch to the storage.
						let write_batch_jh = tokio::task::spawn_blocking({
							let storage = storage.clone();
							move || {storage.write_batch(batch)}
						});
						spawn_result_futures.push(write_batch_jh);
					},

				}
			}
			// Every tick product a new block.
			_ = produce_block_interval.tick() => {
				// Produce only one block at a time.
				// If some is already in production, wait next tick.
				if produce_block_jh.is_none() {
					let produce_block_batch_jh = tokio::task::spawn_blocking({
						let storage = storage.clone();
						move || {storage.produce_next_block()}
					});
					produce_block_jh = Some(produce_block_batch_jh);
				}
			}

			//propagate the new block.
			Some(block) = conditional_block_producing(produce_block_jh.as_mut()) => {
				let block_digest = block.get_block_digest();
				// Send the block to all registered follower

				//send the block to Celestia.
				let celestia_send_jh = tokio::spawn({
					let celestia = celestia.clone();
					async move {celestia.send_block(block_digest).await}
				});
				spawn_result_futures.push(celestia_send_jh);
			}

			Some(Ok(res)) = spawn_result_futures.next() =>  {
				// just log for now, add more logic later.
				if let Err(err) = res {
					tracing::error!("Error during future execution:{err}");
				}
			}
		}
	}
}

/// manage the optional future for block production.
async fn conditional_block_producing(
	fut: Option<&mut JoinHandle<Result<Option<SequencerBlock>, DaSequencerError>>>,
) -> Option<SequencerBlock> {
	match fut {
		Some(fut) => {
			let res = fut.await;
			match res {
				Ok(Ok(Some(res))) => Some(res),
				Ok(Ok(None)) => None,
				Ok(Err(err)) => {
					// for now log the error, TODO better error management.
					tracing::error!("Error during Block producing:{err}");
					None
				}
				Err(err) => {
					tracing::error!("Block producing joinhandle failed to execute:{err}");
					None
				}
			}
		}
		None => None,
	}
}
