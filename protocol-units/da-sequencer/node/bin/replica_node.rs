use anyhow::Context;
use celestia_types::block::Block;
use maptos_execution_util::da_db::DaDB;
use movement_da_sequencer_client::DaSequencerClient;
use movement_da_sequencer_client::GrpcDaSequencerClient;
use movement_da_sequencer_node::storage::{DaSequencerStorage, Storage};
use movement_da_sequencer_proto::StreamReadFromHeightRequest;
use tokio::select;
use tokio::signal::unix::{signal, SignalKind};
use tokio::sync::{mpsc, oneshot};
use tokio::task::JoinHandle;
use tokio_stream::StreamExt;
use tracing::info_span;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
	use tracing_subscriber::EnvFilter;

	tracing_subscriber::fmt()
		.with_env_filter(
			EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
		)
		.init();

	// Signal management
	let mut sigterm = signal(SignalKind::terminate()).context("can't register to SIGTERM.")?;
	let mut sigint = signal(SignalKind::interrupt()).context("can't register to SIGKILL.")?;
	let mut sigquit = signal(SignalKind::quit()).context("can't register to SIGKILL.")?;

	// Define da-sequencer config path
	let dot_movement = dot_movement::DotMovement::try_from_env()?;

	let da_sequencer_config =
		movement_da_sequencer_config::read_da_sequencer_config(&mut dot_movement).await?;

	// Init block storage
	let dotmovement_path = dot_movement.get_path().to_path_buf();
	let db_storage_path = dotmovement_path.join(&da_sequencer_config.db_storage_relative_path);
	let storage = Storage::try_new(&db_storage_path)?;

	// Create da sequencer client to stream block
	let da_client =
		GrpcDaSequencerClient::try_connect(&da_connection_url, stream_heartbeat_interval_sec)
			.await?;

	let (mut blocks_from_da, mut alert_channel) = da_client
		.stream_read_from_height(StreamReadFromHeightRequest { height: synced_height })
		.await
		.map_err(|e| {
			tracing::error!("Failed to stream blocks from DA: {:?}", e);
			e
		})?;

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
								let da_block_height = da_block.height;

						let span = info_span!(target: "movement_replicat", "process_block_from_da", block_id = %hex::encode(da_block.block_id.clone()));
						tracing::info!("Receive block from DA: {:?}",da_block.node_state);
						let block: Block = bcs::from_bytes(&da_block.data[..])?;
					}
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
			else => break,
		}
	}
	anyhow::bail!("Block execution loop break. Node need to be restarted.")
}
