use crate::node::{da_db::DaDB, tasks};
use maptos_dof_execution::MakeOptFinServices;
use maptos_dof_execution::{v1::Executor, DynOptFinExecutor};
use maptos_opt_executor::executor::TxExecutionResult;
use mcr_settlement_client::McrSettlementClient;
use mcr_settlement_manager::CommitmentEventStream;
use mcr_settlement_manager::McrSettlementManager;
use movement_config::Config;
use movement_rest::MovementRest;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

use anyhow::Context;
//use tokio::try_join;
use tracing::debug;

pub struct MovementPartialNode<T> {
	executor: T,
	settlement_manager: Option<McrSettlementManager>,
	commitment_events: Option<CommitmentEventStream>,
	movement_rest: MovementRest,
	config: Config,
	da_db: DaDB,
}

impl<T> MovementPartialNode<T>
where
	T: DynOptFinExecutor + Send + 'static,
{
	pub fn settlement_manager(&self) -> &Option<McrSettlementManager> {
		&self.settlement_manager
	}

	pub fn executor(&self) -> &T {
		&self.executor
	}

	// ! Currently this only implements opt.
	/// Runs the executor until crash or shutdown.
	pub async fn run(
		self,
		mempool_commit_tx_receiver: UnboundedReceiver<Vec<TxExecutionResult>>,
	) -> Result<(), anyhow::Error> {
		let (context, exec_background) = self
			.executor
			.background(mempool_commit_tx_receiver, &self.config.execution_config.maptos_config)?;
		let services = context.services();
		let mut movement_rest = self.movement_rest;
		movement_rest.set_context(services.opt_api_context());
		let exec_settle_task = tasks::execute_settle::Task::new(
			self.executor,
			self.settlement_manager,
			self.da_db,
			self.commitment_events,
			self.config.execution_extension.clone(),
			self.config.mcr.clone(),
		);

		let (result, _index, _remaining) = futures::future::select_all(vec![
			tokio::spawn({
				let da_signer = self
					.config
					.execution_config
					.maptos_config
					.da_sequencer
					.batch_signer_identifier
					.clone();

				let propagate_execution_state = self
					.config
					.execution_config
					.maptos_config
					.da_sequencer
					.propagate_execution_state;

				let da_sequencer_url =
					self.config.execution_config.maptos_config.da_sequencer.connection_url.clone();
				let stream_heartbeat_interval_sec = self
					.config
					.execution_config
					.maptos_config
					.da_sequencer
					.stream_heartbeat_interval_sec;

				async move {
					exec_settle_task
						.run(
							da_sequencer_url,
							stream_heartbeat_interval_sec,
							self.config.da_db.allow_sync_from_zero,
							propagate_execution_state,
							&da_signer,
						)
						.await
				}
			}),
			tokio::spawn(exec_background),
			tokio::spawn(services.run()),
		])
		.await;
		result??;
		Ok(())
	}
}

impl MovementPartialNode<Executor> {
	pub async fn try_executor_from_config(
		config: Config,
		mempool_tx_exec_result_sender: UnboundedSender<Vec<TxExecutionResult>>,
	) -> Result<Executor, anyhow::Error> {
		let executor = Executor::try_from_config(
			config.execution_config.maptos_config.clone(),
			mempool_tx_exec_result_sender,
		)
		.await
		.context("Failed to create the inner executor")?;
		Ok(executor)
	}

	pub async fn try_from_config(
		config: Config,
		mempool_tx_exec_result_sender: UnboundedSender<Vec<TxExecutionResult>>,
	) -> Result<Self, anyhow::Error> {
		debug!("Creating the executor");
		let executor = Executor::try_from_config(
			config.execution_config.maptos_config.clone(),
			mempool_tx_exec_result_sender,
		)
		.await
		.context("Failed to create the inner executor")?;

		let (settlement_manager, commitment_events) = if config.mcr.should_settle() {
			debug!("Creating the settlement client");
			let settlement_client = McrSettlementClient::build_with_config(&config.mcr)
				.await
				.context("Failed to build MCR settlement client with config")?;
			let (settlement_manager, commitment_events) =
				McrSettlementManager::new(settlement_client, &config.mcr);
			(Some(settlement_manager), Some(commitment_events))
		} else {
			(None, None)
		};

		debug!("Creating the movement rest service");
		let movement_rest =
			MovementRest::try_from_env().context("Failed to create MovementRest")?;

		debug!("Creating the DA DB");
		let da_db =
			DaDB::open(&config.da_db.da_db_path).context("Failed to create or get DA DB")?;

		// FIXME: the config value is probably misplaced
		da_db
			.initialize_synced_height(
				config.celestia_da_light_node.celestia_da_light_node_config.initial_height,
			)
			.await?;

		Ok(Self { executor, settlement_manager, commitment_events, movement_rest, config, da_db })
	}
}
