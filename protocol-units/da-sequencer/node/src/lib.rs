use crate::block::SequencerBlock;
use crate::celestia::mock::CelestiaMock;
use crate::celestia::DaSequencerExternalDa;
use crate::error::DaSequencerError;
use crate::server::run_server;
use crate::server::GrpcRequests;
use crate::server::ProducedData;
use crate::storage::DaSequencerStorage;
use crate::storage::Storage;
use crate::whitelist::Whitelist;
use anyhow::Context;
use futures::future::Either;
use futures::stream::FuturesUnordered;
use godfig::{backend::config_file::ConfigFile, Godfig};
use movement_da_sequencer_config::DaSequencerConfig;
use tokio::signal::unix::signal;
use tokio::signal::unix::SignalKind;
use tokio::sync::{mpsc, oneshot};
use tokio::task::JoinHandle;
use tokio_stream::StreamExt;

pub mod batch;
pub mod block;
pub mod celestia;
pub mod error;
mod healthcheck;
pub mod server;
pub mod storage;
#[cfg(test)]
pub mod tests;
pub mod whitelist;

pub const GRPC_REQUEST_CHANNEL_SIZE: usize = 1000;

pub async fn start(mut dot_movement: dot_movement::DotMovement) -> Result<(), anyhow::Error> {
	let pathbuff = movement_da_sequencer_config::get_config_path(&dot_movement);
	tracing::info!("Start Da Sequencer with config file in {pathbuff:?}.");
	dot_movement.set_path(pathbuff);

	let config_file = dot_movement.try_get_or_create_config_file().await?;

	// Get a matching godfig object
	let godfig: Godfig<DaSequencerConfig, ConfigFile> =
		Godfig::new(ConfigFile::new(config_file), vec![]);
	let da_sequencer_config: DaSequencerConfig = godfig.try_wait_for_ready().await?;

	let dotmovement_path = dot_movement.get_path().to_path_buf();

	// Initialize whitelist
	let whitelist_path = dotmovement_path.join(&da_sequencer_config.whitelist_relative_path);
	let whitelist = Whitelist::from_file_and_spawn_reload_thread(whitelist_path)?;

	let (request_tx, request_rx) = mpsc::channel(GRPC_REQUEST_CHANNEL_SIZE);
	// Start gprc server
	let grpc_address = da_sequencer_config.grpc_listen_address;
	let verifying_key = da_sequencer_config.get_main_node_verifying_key()?;

	let grpc_jh = tokio::spawn(async move {
		run_server(grpc_address, request_tx, whitelist, verifying_key).await
	});

	// Start healthcheck entry point
	let healthcheck_url = format!(
		"{}:{}",
		healthcheck::DEFAULT_REST_LISTENER_HOSTNAME,
		da_sequencer_config.healthcheck_bind_port
	);
	let (rest_health_tx, rest_health_rx) = tokio::sync::mpsc::channel(10);
	let rest_service = healthcheck::HealthCheckRest::new(healthcheck_url, rest_health_tx)?;
	let rest_service_future = rest_service.run_service();
	let rest_jh = tokio::spawn(rest_service_future);

	//Start the main loop
	let db_storage_path = dotmovement_path.join(&da_sequencer_config.db_storage_relative_path);

	let storage = Storage::try_new(&db_storage_path)?;

	// TODO Use Celestia Mock for now
	let celestia_mock = CelestiaMock::new();
	let loop_jh =
		tokio::spawn(run(da_sequencer_config, request_rx, rest_health_rx, storage, celestia_mock));

	let (stop_tx, mut stop_rx) = tokio::sync::watch::channel(());
	tokio::spawn({
		let mut sigterm = signal(SignalKind::terminate()).context("can't register to SIGTERM.")?;
		let mut sigint = signal(SignalKind::interrupt()).context("can't register to SIGKILL.")?;
		let mut sigquit = signal(SignalKind::quit()).context("can't register to SIGKILL.")?;
		async move {
			loop {
				tokio::select! {
					_ = sigterm.recv() => (),
					_ = sigint.recv() => (),
					_ = sigquit.recv() => (),
				};
				tracing::info!("Receive Terminate Signal");
				if let Err(err) = stop_tx.send(()) {
					tracing::warn!("Can't update stop watch channel because :{err}");
					return Err::<(), anyhow::Error>(anyhow::anyhow!(err));
				}
			}
		}
	});
	// Use tokio::select! to wait for either the handle or a cancellation signal
	tokio::select! {
		_ = stop_rx.changed() =>(),
		res = grpc_jh => {
			tracing::error!("Grpc server exit because :{res:?}");
		}
		res = rest_jh => {
			tracing::error!("Da Sequencer Rest entry point stops because :{res:?}");
		}
		res = loop_jh => {
			tracing::error!("Da Sequencer main process exit because :{res:?}");
		}
	};

	Ok(())
}

/// Run Da sequencing loop.
/// Runs the DA sequencing loop.
///
/// This function only returns if a critical error occurs, indicating a node crash or unrecoverable failure.
pub async fn run<D, S>(
	config: DaSequencerConfig,
	mut request_rx: mpsc::Receiver<GrpcRequests>,
	mut check_request_rx: mpsc::Receiver<oneshot::Sender<bool>>,
	storage: S,
	celestia: D,
) -> Result<(), DaSequencerError>
where
	D: DaSequencerExternalDa + Clone + Send + 'static,
	S: DaSequencerStorage + Clone + Send + 'static,
{
	let mut produce_block_interval = tokio::time::interval(tokio::time::Duration::from_millis(
		config.block_production_interval_millisec,
	));
	let mut da_stream_heartbeat_interval = tokio::time::interval(tokio::time::Duration::from_secs(
		config.stream_heartbeat_interval_sec,
	));
	let mut spawn_result_futures = FuturesUnordered::new();
	let mut produce_block = false;
	let mut produce_block_jh: Option<JoinHandle<Result<Option<SequencerBlock>, DaSequencerError>>> =
		None; //get_pending_future();
	let mut connected_grpc_sender = vec![];
	let mut current_node_state = None;
	// Batch timestamp should always be greater strict to the last one.
	let mut last_batch_timestamp = chrono::Utc::now().timestamp_micros() as u64;

	let pending = futures::future::pending();
	tokio::pin!(pending);

	loop {
		// Construct a future of for optional block production type using `Either`
		let mut produce_block_fut = match &mut produce_block_jh {
			Some(produce) => Either::Left(produce),
			None => Either::Right(&mut pending),
		};

		tokio::select! {
			// Manage health check request.
			Some(oneshot_tx) = check_request_rx.recv() => {
				//Basic monitoring, always true if the loop run.
				if let Err(err) = oneshot_tx.send(true){
					tracing::warn!("Heal check oneshot channel closed abnormally :{err:?}");
				}
			}

			// Manage grpc request.
			Some(grpc_request) = request_rx.recv() => {
				match grpc_request {
					GrpcRequests::StartBlockStream(proposed_block_tx, curent_height_callback) => {
						connected_grpc_sender.push(proposed_block_tx);

						// Send back the current height.
						let start_jh = tokio::task::spawn_blocking({
							let storage = storage.clone();
							move || {
								let current_height = storage.get_current_block_height()?;
								let _ = curent_height_callback.send(current_height);
								Ok::<(), DaSequencerError>(())
						}});
						spawn_result_futures.push(start_jh);
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
						// Create an unique batch data
						let batch = batch.unique(last_batch_timestamp);
						last_batch_timestamp = batch.data().timestamp;

						//send batch to the storage.
						let write_batch_jh = tokio::task::spawn_blocking({
							let storage = storage.clone();
							move || {storage.write_batch(batch)}
						});
						spawn_result_futures.push(write_batch_jh);
					},
					GrpcRequests::SendState(state) => current_node_state = Some(state),

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
					produce_block_jh = Some(produce_block_batch_jh);
					produce_block = true;
				}
			}

			//propagate the new block.
			res = &mut produce_block_fut => {
				produce_block_jh = None;
				produce_block = false;
				match res {
					Ok(Ok(Some(block))) => {
						let block_id = block.id();
						// Send the block to all registered follower
						// For now send to the main loop because there are very few followers (<100).
						tracing::info!(sender_len = %connected_grpc_sender.len(), block_height= %block.height().0, "New block produced, sent to fullnodes.");
						stream_block_to_sender(&mut connected_grpc_sender, ProducedData::Block(block, current_node_state.clone())).await;

						//send the block to Celestia.
						let celestia_send_jh = tokio::spawn({
							let celestia = celestia.clone();
							async move {celestia.send_block(block_id).await}
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
				stream_block_to_sender(&mut connected_grpc_sender, ProducedData::HeartBeat).await;

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
	senders: &mut Vec<mpsc::UnboundedSender<ProducedData>>,
	data: ProducedData,
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
