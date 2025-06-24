use super::Executor;
use crate::background::BackgroundTask;
use crate::executor::TxExecutionResult;
use crate::executor::EXECUTOR_CHANNEL_SIZE;
use crate::{bootstrap, Context};
use anyhow::Context as _;
use aptos_config::config::NodeConfig;
#[cfg(test)]
use aptos_crypto::ed25519::Ed25519PrivateKey;
use aptos_crypto::ed25519::Ed25519PublicKey;
#[cfg(test)]
use aptos_crypto::ValidCryptoMaterialStringExt;
use aptos_executor::block_executor::BlockExecutor;
use aptos_mempool::MempoolClientRequest;
use dot_movement::DotMovement;
use futures::channel::mpsc as futures_mpsc;
use maptos_execution_util::config::Config;
use movement_collections::garbage::{counted::GcCounter, Duration};
use movement_signer::cryptography::ed25519::Ed25519;
use movement_signer::Signing;
#[cfg(test)]
use movement_signer_loader::identifiers::{local::Local, SignerIdentifier};
use movement_signer_loader::{Load, LoadedSigner};
use std::net::ToSocketAddrs;
use std::sync::{Arc, RwLock};
use tempfile::TempDir;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

impl Executor {
	pub fn bootstrap_with_public_key(
		maptos_config: &Config,
		public_key: Ed25519PublicKey,
		mempool_tx_exec_result_sender: UnboundedSender<Vec<TxExecutionResult>>,
	) -> Result<Self, anyhow::Error> {
		// get dot movement
		// todo: this is a slight anti-pattern, but it's fine for now
		let dot_movement = DotMovement::try_from_env()?;

		// set up the node config
		let mut node_config = NodeConfig::default();

		// read-only settings
		if maptos_config.chain.maptos_read_only {
			node_config.api.transaction_submission_enabled = false;
			node_config.api.encode_submission_enabled = false;
			node_config.api.transaction_simulation_enabled = false;
			node_config.api.gas_estimation.enabled = false;
			node_config.api.periodic_gas_estimation_ms = None;
		}

		// pruning config
		node_config.storage.storage_pruner_config.ledger_pruner_config.enable =
			maptos_config.chain.enabled_pruning;
		node_config.storage.storage_pruner_config.ledger_pruner_config.prune_window =
			maptos_config.chain.maptos_ledger_prune_window;

		node_config.storage.storage_pruner_config.state_merkle_pruner_config.enable =
			maptos_config.chain.enabled_pruning;
		node_config
			.storage
			.storage_pruner_config
			.state_merkle_pruner_config
			.prune_window = maptos_config.chain.maptos_state_merkle_prune_window;

		node_config.storage.storage_pruner_config.epoch_snapshot_pruner_config.enable =
			maptos_config.chain.enabled_pruning;
		node_config
			.storage
			.storage_pruner_config
			.epoch_snapshot_pruner_config
			.prune_window = maptos_config.chain.maptos_epoch_snapshot_prune_window;

		node_config.indexer.enabled = true;
		// indexer config
		node_config.indexer.postgres_uri =
			Some(maptos_config.indexer_processor.postgres_connection_string.clone());
		node_config.indexer.processor = Some("default_processor".to_string());
		node_config.indexer.check_chain_id = Some(false);
		node_config.indexer.skip_migrations = Some(false);
		node_config.indexer.fetch_tasks = Some(4);
		node_config.indexer.processor_tasks = Some(4);
		node_config.indexer.emit_every = Some(4);
		node_config.indexer.batch_size = Some(8);
		node_config.indexer.gap_lookback_versions = Some(4);

		node_config.indexer_grpc.enabled = true;

		// indexer_grpc config
		node_config.indexer_grpc.processor_batch_size = 4;
		node_config.indexer_grpc.processor_task_count = 4;
		node_config.indexer_grpc.output_batch_size = 4;
		node_config.indexer_grpc.address = (
			maptos_config.indexer.maptos_indexer_grpc_listen_hostname.as_str(),
			maptos_config.indexer.maptos_indexer_grpc_listen_port,
		)
			.to_socket_addrs()?
			.next()
			.context("failed to resolve the value of maptos_indexer_grpc_listen_hostname")?;
		node_config.indexer_grpc.use_data_service_interface = true;

		// indexer table info config
		node_config.indexer_table_info.enabled = true;
		node_config.storage.dir = dot_movement.get_path().join("maptos-storage");
		node_config.storage.set_data_dir(node_config.storage.dir.clone());

		let known_release = aptos_framework_known_release::KnownRelease::try_new(
			maptos_config.chain.known_framework_release_str.as_str(),
		)?;
		let (db, signer) = bootstrap::maybe_bootstrap_empty_db(
			&node_config,
			maptos_config.chain.maptos_db_path.as_ref().context("No db path provided.")?,
			maptos_config.chain.maptos_chain_id.clone(),
			&public_key,
			&known_release,
		)?;

		Ok(Self {
			mempool_tx_exec_result_sender,
			block_executor: Arc::new(BlockExecutor::new(db.clone())),
			signer,
			transactions_in_flight: Arc::new(RwLock::new(GcCounter::new(
				Duration::try_new(maptos_config.mempool.sequence_number_ttl_ms)?,
				Duration::try_new(maptos_config.mempool.gc_slot_duration_ms)?,
			))),
			config: maptos_config.clone(),
			node_config: node_config.clone(),
		})
	}

	pub async fn bootstrap(
		maptos_config: &Config,
		mempool_tx_exec_result_sender: UnboundedSender<Vec<TxExecutionResult>>,
	) -> Result<Self, anyhow::Error> {
		let loader: LoadedSigner<Ed25519> =
			maptos_config.chain.maptos_private_key_signer_identifier.load().await?;
		let public_key = Ed25519PublicKey::try_from(loader.public_key().await?.as_bytes())?;

		Self::bootstrap_with_public_key(maptos_config, public_key, mempool_tx_exec_result_sender)
	}

	pub async fn try_from_config(
		maptos_config: Config,
		mempool_tx_exec_result_sender: UnboundedSender<Vec<TxExecutionResult>>,
	) -> Result<Self, anyhow::Error> {
		Self::bootstrap(&maptos_config, mempool_tx_exec_result_sender).await
	}

	#[cfg(test)]
	pub fn try_test_default_with_public_key_bytes(
		public_key_bytes: &[u8],
		mempool_tx_exec_result_sender: UnboundedSender<Vec<TxExecutionResult>>,
	) -> Result<(Self, TempDir), anyhow::Error> {
		use aptos_crypto::ValidCryptoMaterialStringExt;

		let public_key =
			Ed25519PublicKey::from_encoded_string(hex::encode(public_key_bytes).as_str())?;
		Self::try_test_default_with_public_key(public_key, mempool_tx_exec_result_sender)
	}

	pub fn try_test_default_with_public_key(
		public_key: Ed25519PublicKey,
		mempool_tx_exec_result_sender: UnboundedSender<Vec<TxExecutionResult>>,
	) -> Result<(Self, TempDir), anyhow::Error> {
		let tempdir = tempfile::tempdir()?;

		let mut maptos_config = Config::default();
		maptos_config.chain.maptos_db_path.replace(tempdir.path().to_path_buf());
		let executor = Self::bootstrap_with_public_key(
			&maptos_config,
			public_key,
			mempool_tx_exec_result_sender,
		)?;
		Ok((executor, tempdir))
	}

	#[cfg(test)]
	pub async fn try_test_default(
		private_key: Ed25519PrivateKey,
		mempool_tx_exec_result_sender: UnboundedSender<Vec<TxExecutionResult>>,
	) -> Result<(Self, TempDir), anyhow::Error> {
		let tempdir = tempfile::tempdir()?;

		let mut maptos_config = Config::default();
		let raw_private_key_hex = private_key.to_encoded_string()?.to_string();
		let prefix_stripped =
			raw_private_key_hex.strip_prefix("0x").unwrap_or(&raw_private_key_hex);
		maptos_config.chain.maptos_private_key_signer_identifier =
			SignerIdentifier::Local(Local { private_key_hex_bytes: prefix_stripped.to_string() });

		// replace the db path with the temporary directory
		maptos_config.chain.maptos_db_path.replace(tempdir.path().to_path_buf());
		let executor = Self::try_from_config(maptos_config, mempool_tx_exec_result_sender).await?;
		Ok((executor, tempdir))
	}

	// 	pub async fn try_generated() -> Result<
	// 	(
	// 		Self,
	// 		TempDir,
	// 		Ed25519PrivateKey,
	// 		futures::channel::mpsc::Receiver<Vec<TxExecutionResult>>,
	// 	),
	// 	anyhow::Error,
	// > {
	// 	// generate a random private key
	// 	let private_key = Ed25519PrivateKey::generate_for_testing();

	// 	// generate a sender
	// 	let (mempool_tx_exec_result_sender, receiver) =
	// 		futures_mpsc::channel::<Vec<TxExecutionResult>>(EXECUTOR_CHANNEL_SIZE);
	// 	let tempdir = tempfile::tempdir()?;

	// 	let mut maptos_config = Config::default();
	// 	let raw_private_key_hex = private_key.to_encoded_string()?.to_string();
	// 	let prefix_stripped =
	// 		raw_private_key_hex.strip_prefix("0x").unwrap_or(&raw_private_key_hex);
	// 	maptos_config.chain.maptos_private_key_signer_identifier =
	// 		SignerIdentifier::Local(Local { private_key_hex_bytes: prefix_stripped.to_string() });

	// 	// replace the db path with the temporary directory
	// 	maptos_config.chain.maptos_db_path.replace(tempdir.path().to_path_buf());
	// 	let executor = Self::try_from_config(maptos_config, mempool_tx_exec_result_sender).await?;
	// 	Ok((executor, tempdir, private_key, receiver))
	// }

	/// Creates an instance of [`Context`] and the background [`TransactionPipe`]
	/// task to process transactions. If the configuration is for a read-only node,
	/// `None` is returned instead of the transaction pipe task.
	/// The `Context` must be kept around for as long as the `TransactionPipe`
	/// task needs to be running.
	pub fn background(
		&self,
		mempool_commit_tx_receiver: UnboundedReceiver<Vec<TxExecutionResult>>,
		mempool_request_sender: futures_mpsc::Sender<MempoolClientRequest>,
	) -> anyhow::Result<(Context, BackgroundTask)> {
		let node_config = self.node_config.clone();
		let maptos_config = self.config.clone();

		let da_batch_signer = maptos_config.da_sequencer.batch_signer_identifier.clone();

		let background_task = if maptos_config.chain.maptos_read_only {
			// use the default signer, block executor, and mempool
			//TODO correct the mempool_client_sender not used.
			let (_mempool_client_sender, mempool_client_receiver) =
				futures_mpsc::channel::<MempoolClientRequest>(EXECUTOR_CHANNEL_SIZE);
			BackgroundTask::read_only(mempool_client_receiver)
		} else {
			BackgroundTask::transaction_pipe(
				mempool_commit_tx_receiver,
				self.db().reader.clone(),
				&node_config,
				&self.config.mempool,
				&self.config.access_control,
				self.transactions_in_flight.clone(),
				maptos_config.load_shedding.max_transactions_in_flight,
				da_batch_signer,
			)?
		};

		let cx =
			Context::new(self.db().clone(), mempool_request_sender, maptos_config, node_config);

		Ok((cx, background_task))
	}
}
