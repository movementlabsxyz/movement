use anyhow::Context;
use futures::stream::FuturesUnordered;
use movement_da_sequencer_client::DaSequencerClient;
use movement_da_sequencer_client::GrpcDaSequencerClient;
use movement_da_sequencer_config::DaReplicatConfig;
use movement_da_sequencer_node::block::BlockHeight;
use movement_da_sequencer_node::block::NodeState;
use movement_da_sequencer_node::block::SequencerBlock;
use movement_da_sequencer_node::error::DaSequencerError;
use movement_da_sequencer_node::server::run_server;
use movement_da_sequencer_node::server::GrpcRequests;
use movement_da_sequencer_node::server::ProducedData;
use movement_da_sequencer_node::storage::{DaSequencerStorage, Storage};
use movement_da_sequencer_node::whitelist::Whitelist;
use movement_da_sequencer_node::GRPC_REQUEST_CHANNEL_SIZE;
use movement_da_sequencer_proto::BatchWriteRequest;
use movement_da_sequencer_proto::StreamReadFromHeightRequest;
use movement_signer::cryptography::ed25519::Ed25519;
use movement_signer_loader::{Load, LoadedSigner};
use movement_types::block::Block;
use tokio::select;
use tokio::signal::unix::{signal, SignalKind};
use tokio::sync::mpsc;
use tokio_stream::StreamExt;

pub async fn start(mut dot_movement: dot_movement::DotMovement) -> Result<(), anyhow::Error> {
	// Signal management
	let mut sigterm = signal(SignalKind::terminate()).context("can't register to SIGTERM.")?;
	let mut sigint = signal(SignalKind::interrupt()).context("can't register to SIGKILL.")?;
	let mut sigquit = signal(SignalKind::quit()).context("can't register to SIGKILL.")?;

	let da_sequencer_config = DaReplicatConfig::try_from_env(&mut dot_movement).await?;

	// Init block storage
	let dotmovement_path = dot_movement.get_path().to_path_buf();
	let db_storage_path =
		dotmovement_path.join(&da_sequencer_config.da_sequencer.db_storage_relative_path);
	let storage = Storage::try_new(&db_storage_path)?;

	// Create da sequencer client to stream block

	//Connect to the main DA sequencer to get all missing block and produced one.
	let mut da_client =
		GrpcDaSequencerClient::try_connect(&da_sequencer_config.da_client.connection_url, 10)
			.await?;

	let last_synced_height = storage.get_current_block_height()? + 1;
	let (mut blocks_from_da, mut alert_channel) = da_client
		.stream_read_from_height(StreamReadFromHeightRequest { height: last_synced_height.into() })
		.await
		.map_err(|e| {
			tracing::error!("Failed to stream blocks from DA: {:?}", e);
			e
		})?;

	//start grpc entry point
	// Initialize whitelist
	let whitelist_path =
		dotmovement_path.join(&da_sequencer_config.da_sequencer.whitelist_relative_path);
	let whitelist = Whitelist::from_file_and_spawn_reload_thread(whitelist_path)?;

	let (request_tx, mut request_rx) = mpsc::channel(GRPC_REQUEST_CHANNEL_SIZE);
	// Start gprc server
	let grpc_address = da_sequencer_config.da_sequencer.grpc_listen_address;
	let verifying_key = da_sequencer_config.da_sequencer.get_main_node_verifying_key()?;

	let mut grpc_jh = tokio::spawn(async move {
		run_server(grpc_address, request_tx, whitelist, verifying_key).await
	});

	// Load batch signer
	let da_batch_signer = da_sequencer_config.da_client.batch_signer_identifier.clone();

	// Start healthcheck entry point
	let healthcheck_url =
		format!("0.0.0.0:{}", da_sequencer_config.da_sequencer.healthcheck_bind_port);
	let (rest_health_tx, mut rest_health_rx) = tokio::sync::mpsc::channel(10);
	let rest_service = movement_da_sequencer_node::healthcheck::HealthCheckRest::new(
		healthcheck_url,
		rest_health_tx,
	)?;
	let rest_service_future = rest_service.run_service();
	let mut rest_jh = tokio::spawn(rest_service_future);

	// Some processing vars
	let mut spawn_result_futures = FuturesUnordered::new();
	let mut connected_grpc_sender = vec![];
	let mut da_stream_heartbeat_interval = tokio::time::interval(tokio::time::Duration::from_secs(
		da_sequencer_config.da_sequencer.stream_heartbeat_interval_sec,
	));

	loop {
		select! {
			next_block = blocks_from_da.next() => {
				match next_block {
					None => {
						tracing::error!("Da stream return none, stream broken");
						break;
					}
					Some(res) => {
						let da_block = res.context("failed to get next block from DA")?;
						let da_block_height: BlockHeight = da_block.height.into();
						let node_state: Option<NodeState> = da_block.node_state.as_ref().map(|state| state.into());

						tracing::info!("Receive block at height from DA: {:?}", da_block_height);
						let block: Block = bcs::from_bytes(&da_block.data[..])?;
						let sequencer_block = SequencerBlock::new(da_block_height, block);
						//save the block
						let start_jh = tokio::task::spawn_blocking({
							let storage = storage.clone();
							let sequencer_block = sequencer_block.clone();
							move || {
								storage.save_block(&sequencer_block, None)
						}});
						spawn_result_futures.push(start_jh);

						// Send the block to all registered full node
						// For now send to the main loop because there are very few followers (<100).
						tracing::info!(sender_len = %connected_grpc_sender.len(), block_height= %sequencer_block.height().0, "New block produced, sent to fullnodes.");

						stream_block_to_sender(&mut connected_grpc_sender, ProducedData::Block(sequencer_block, node_state.clone())).await;
					}
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
						// All batch verification has been done, propagate the batch to the da-sequencer with replicat key
						if !batch.data().0.is_empty() {
							// Build batch and submit request.
							tracing::info!("Propagate new batch with {} txs.", batch.data().0.len());
							let loader: LoadedSigner<Ed25519> = da_batch_signer.load().await?;

							//send the batch in a separate task to avoid to slow the loop.
							let handle = tokio::spawn({
								let mut client = da_client.clone();
								async move {
									let batch_bytes = bcs::to_bytes(&batch.data().0).expect("Serialization failed");
									let encoded =
										movement_da_sequencer_client::sign_and_encode_batch(batch_bytes, &loader)
											.await
											.unwrap();
									client.batch_write(BatchWriteRequest { data: encoded }).await.map_err(|status| {
										tracing::warn!("Send Batch to Da failed because: {status:?}");
										DaSequencerError::SendFailure}
									)?;
									Ok(())
								}
							});
							spawn_result_futures.push(handle);
						}

					},
					GrpcRequests::SendState(_state) => (), // can't send node state with replicat.

				}
			}
			// Every tick will produce a heartbeat.
			_ = da_stream_heartbeat_interval.tick() => {
				tracing::info!(sender_len = %connected_grpc_sender.len(), "Produced a heartbeat, sent to fullnodes");
				stream_block_to_sender(&mut connected_grpc_sender, ProducedData::HeartBeat).await;

			}
			// Manage health check request.
			Some(oneshot_tx) = rest_health_rx.recv() => {
				//Basic monitoring, always true if the loop run.
				if let Err(err) = oneshot_tx.send(true){
					tracing::warn!("Heal check oneshot channel closed abnormally :{err:?}");
				}
			}
			_ = alert_channel.recv() => {
				tracing::error!("Da client stream channel timeout because it's idle. Exit");
				break;
			}
			_ = sigterm.recv() => {
				tracing::error!("Reveived sigterm, exiting");
				break;
			},
			_ = sigint.recv() => {
				tracing::error!("Reveived sigint, exiting");
				break;
			}
			_ = sigquit.recv() => {
				tracing::error!("Reveived sigquit, exiting");
				break;
			}
			// Manage futures result.
			Some(Ok(res)) = spawn_result_futures.next() =>  {
				// just log for now, add more logic later.
				if let Err(err) = res {
					tracing::error!(error = %err, "Error during future execution.");
				}
			}
			res = &mut grpc_jh => {
				tracing::error!("Grpc server exit because :{res:?}");
				break;
			}
			res = &mut rest_jh => {
				tracing::error!("Health check server exit because :{res:?}");
				break;
			}

			else => break,
		}
	}
	anyhow::bail!("Block execution loop break. Node need to be restarted.")
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
