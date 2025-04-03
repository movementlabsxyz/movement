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
		config.block_production_interval_millisec,
	));
	let mut da_stream_heartbeat_interval = tokio::time::interval(tokio::time::Duration::from_secs(
		config.stream_heartbeat_interval_sec,
	));
	let mut spawn_result_futures = FuturesUnordered::new();
	let mut produce_block = false;
	let mut produce_block_jh = get_pending_future();
	let mut connected_grpc_sender = vec![];

	loop {
		tokio::select! {
			// Manage grpc request.
			Some(grpc_request) = request_rx.recv() => {
				match grpc_request {
					GrpcRequests::StartBlockStream(produced_tx, curent_height_callback) => {
						connected_grpc_sender.push(produced_tx);

						// Send back the current height.
						let _ = tokio::task::spawn_blocking({
							let storage = storage.clone();
							move || {
								let current_height = storage.get_current_block_height();
								let _ = curent_height_callback.send(current_height);
						}}).await;
					},
					GrpcRequests::GetBlockHeight(block_height, callback) => {
						let get_block_jh = tokio::task::spawn_blocking({
							let storage = storage.clone();
							move || {storage.get_block_at_height(block_height)}
						});
						tokio::spawn(async move {
							let result = get_block_jh.await;
							// Manage result.
							let to_send = match result {
								Err(err) => {
									tracing::error!(error = %err, "spawn_blocking task failed.");
									None
								}
								Ok(Err(err)) => {
									tracing::error!(error = %err, "Storage get_block_at_height return an error.");
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
				if !produce_block {
					let produce_block_batch_jh = tokio::task::spawn_blocking({
						let storage = storage.clone();
						move || {storage.produce_next_block()}
					});
					produce_block_jh = produce_block_batch_jh;
					produce_block = true;
				}
			}

			//propagate the new block.
			res = &mut produce_block_jh => {
				produce_block_jh = get_pending_future();
				produce_block = false;
				match res {
					Ok(Ok(Some(block))) => {
						let block_digest = block.get_block_digest();
						// Send the block to all registered follower
						// For now send to the main loop because there are very few followers (<100).
						tracing::info!(sender_len = %connected_grpc_sender.len(), block_height= %block.height.0, "New block produced, send to fullnodes.");
						stream_block_to_sender(&mut connected_grpc_sender, Some(block)).await;

						//send the block to Celestia.
						let celestia_send_jh = tokio::spawn({
							let celestia = celestia.clone();
							async move {celestia.send_block(block_digest).await}
						});
						spawn_result_futures.push(celestia_send_jh);
					},
					Ok(Ok(None)) => (),
					Ok(Err(err)) => {
						// for now log the error, TODO better error management.
						tracing::error!("Error during Block producing:{err}");
						// TODO manage DB error. see issue 1173
					}
					Err(err) => {
						tracing::error!("Block producing joinhandle failed to execute:{err}");
						// TODO manage tokio error. see issue 1173
					}
				}
			}
			// Every tick will produce a heartbeat.
			_ = da_stream_heartbeat_interval.tick() => {
				tracing::info!(sender_len = %connected_grpc_sender.len(), "Produced a heartbeat, sent to fullnodes");
				stream_block_to_sender(&mut connected_grpc_sender, None).await;

			}

			// Manage futures result.
			Some(Ok(res)) = spawn_result_futures.next() =>  {
				// just log for now, add more logic later.
				if let Err(err) = res {
					tracing::error!(error = %err, "Error during future execution.");
				}
			}
		}
	}
}

async fn stream_block_to_sender(
	senders: &mut Vec<mpsc::UnboundedSender<Option<SequencerBlock>>>,
	data: Option<SequencerBlock>,
) {
	let mut new_sender = vec![];
	for sender in senders.drain(..) {
		// Remove the sender in error because it means the client was disconnected.
		if let Err(err) = sender.send(data.clone()) {
			tracing::warn!("Failed to send block to grpc client. Client disconnected. remove connection :{err}");
		} else {
			new_sender.push(sender);
		}
	}
	*senders = new_sender;
}

fn get_pending_future() -> JoinHandle<Result<Option<SequencerBlock>, DaSequencerError>> {
	tokio::spawn(futures::future::pending())
}
