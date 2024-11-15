use crate::{da_db::DaDB, tasks};
use m1_da_light_node_client::LightNodeServiceClient;
use maptos_dof_execution::MakeOptFinServices;
use maptos_dof_execution::{v1::Executor, DynOptFinExecutor};
use mcr_settlement_client::McrSettlementClient;
use mcr_settlement_manager::CommitmentEventStream;
use mcr_settlement_manager::McrSettlementManager;
use movement_rest::MovementRest;
use suzuka_config::Config;

use anyhow::Context;
use tokio::sync::mpsc;
use tokio::try_join;
use tracing::debug;

pub struct SuzukaPartialNode<T> {
	executor: T,
	light_node_client: LightNodeServiceClient<tonic::transport::Channel>,
	settlement_manager: McrSettlementManager,
	commitment_events: Option<CommitmentEventStream>,
	movement_rest: MovementRest,
	config: Config,
	da_db: DaDB,
}

impl<T> SuzukaPartialNode<T>
where
	T: DynOptFinExecutor + Send + 'static,
{
	// ! Currently this only implements opt.
	/// Runs the executor until crash or shutdown.
	pub async fn run(self) -> Result<(), anyhow::Error> {
		let (transaction_sender, transaction_receiver) = mpsc::channel(16);
		let (context, exec_background) = self
			.executor
			.background(transaction_sender, &self.config.execution_config.maptos_config)?;
		let services = context.services();
		let mut movement_rest = self.movement_rest;
		movement_rest.set_context(services.opt_api_context());
		let exec_settle_task = tasks::execute_settle::Task::new(
			self.executor,
			self.settlement_manager,
			self.da_db,
			self.light_node_client.clone(),
			self.commitment_events,
			self.config.execution_extension.clone(),
		);
		let transaction_ingress_task = tasks::transaction_ingress::Task::new(
			transaction_receiver,
			self.light_node_client,
			// FIXME: why are the struct member names so tautological?
			self.config.m1_da_light_node.m1_da_light_node_config,
		);

		let (
			execution_and_settlement_result,
			transaction_ingress_result,
			background_task_result,
			services_result,
		) = try_join!(
			tokio::spawn(async move { exec_settle_task.run().await }),
			tokio::spawn(async move { transaction_ingress_task.run().await }),
			tokio::spawn(exec_background),
			tokio::spawn(services.run()),
			// tokio::spawn(async move { movement_rest.run_service().await }),
		)?;
		execution_and_settlement_result
			.and(transaction_ingress_result)
			.and(background_task_result)
			.and(services_result)
	}
}

impl SuzukaPartialNode<Executor> {
	pub async fn try_from_config(config: Config) -> Result<Self, anyhow::Error> {
		// todo: extract into getter
		let light_node_connection_hostname = config
			.m1_da_light_node
			.m1_da_light_node_config
			.m1_da_light_node_connection_hostname();

		// todo: extract into getter
		let light_node_connection_port = config
			.m1_da_light_node
			.m1_da_light_node_config
			.m1_da_light_node_connection_port();
		// todo: extract into getter
		debug!(
			"Connecting to light node at {}:{}",
			light_node_connection_hostname, light_node_connection_port
		);
		let light_node_client = LightNodeServiceClient::connect(format!(
			"http://{}:{}",
			light_node_connection_hostname, light_node_connection_port
		))
		.await
		.context("Failed to connect to light node")?;

		debug!("Creating the executor");
		let executor = Executor::try_from_config(config.execution_config.maptos_config.clone())
			.context("Failed to create the inner executor")?;

		debug!("Creating the settlement client");
		let settlement_client = McrSettlementClient::build_with_config(&config.mcr)
			.await
			.context("Failed to build MCR settlement client with config")?;
		let (settlement_manager, commitment_events) =
			McrSettlementManager::new(settlement_client, &config.mcr);
		let commitment_events =
			if config.mcr.should_settle() { Some(commitment_events) } else { None };

		debug!("Creating the movement rest service");
		let movement_rest =
			MovementRest::try_from_env().context("Failed to create MovementRest")?;

		debug!("Creating the DA DB");
		let da_db =
			DaDB::open(&config.da_db.da_db_path).context("Failed to create or get DA DB")?;

		Ok(Self {
			executor,
			light_node_client,
			settlement_manager,
			commitment_events,
			movement_rest,
			config,
			da_db,
		})
	}
}
