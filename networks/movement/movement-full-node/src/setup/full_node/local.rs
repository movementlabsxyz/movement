use super::MovementFullNodeSetupOperations;
use dot_movement::DotMovement;

// use tracing::debug;

#[derive(Debug, Clone, Default)]
pub struct Local {
	mcr_settlement_strategy: mcr_settlement_setup::Setup,
}

impl Local {
	pub fn new() -> Self {
		Self { mcr_settlement_strategy: Default::default() }
	}

	pub async fn setup_da_sequencer(
		&self,
		dot_movement: DotMovement,
		config: movement_config::Config,
	) -> Result<movement_config::Config, anyhow::Error> {
		// run the maptos execution config setup
		let config = self.setup_maptos_execution_config(dot_movement.clone(), config).await?;
		// run the da_db config setup
		self.setup_da_db_config(dot_movement.clone(), config).await
	}

	async fn run_da_light_node_setup(
		&self,
		dot_movement: DotMovement,
		mut config: movement_config::Config,
	) -> Result<
		(movement_config::Config, tokio::task::JoinHandle<Result<String, anyhow::Error>>),
		anyhow::Error,
	> {
		let da_light_node_config = config.celestia_da_light_node.clone();

		let new_da_light_node_config =
			movement_da_light_node_setup::setup(dot_movement.clone(), da_light_node_config).await?;

		// Update the config with the new da_light_node_config
		config.celestia_da_light_node = new_da_light_node_config;

		tracing::info!("Running mcr_settlement_setup");
		let mcr_settlement_config: mcr_settlement_config::Config = config.mcr.clone();
		let (mcr_config, join_handle) =
			self.mcr_settlement_strategy.setup(&dot_movement, mcr_settlement_config).await?;
		config.mcr = mcr_config;

		Ok((config, join_handle))
	}

	async fn setup_maptos_execution_config(
		&self,
		dot_movement: DotMovement,
		mut config: movement_config::Config,
	) -> Result<movement_config::Config, anyhow::Error> {
		// update the db path
		let chain_id = config.execution_config.maptos_config.chain.maptos_chain_id;
		let db_path = dot_movement
			.get_path()
			.join("maptos")
			.join(chain_id.to_string())
			.join(".maptos");
		config.execution_config.maptos_config.chain.maptos_db_path.replace(db_path);

		// Set as main node that send state.
		let local = std::env::var_os("MAYBE_RUN_LOCAL").unwrap_or("false".into());
		if local == "false" {
			config.execution_config.maptos_config.da_sequencer.propagate_execution_state = false;
		} else {
			config.execution_config.maptos_config.da_sequencer.propagate_execution_state = true;
		}

		// write the maptos signer address to the default signer address whitelist
		let default_signer_address_whitelist_path =
			dot_movement.get_path().join("default_signer_address_whitelist");

		std::fs::write(
			default_signer_address_whitelist_path.clone(),
			format!("{}", "000000000000000000000000000000000000000000000000000000000a550c18"),
		)?;

		Ok(config)
	}

	async fn setup_da_db_config(
		&self,
		dot_movement: DotMovement,
		mut config: movement_config::Config,
	) -> Result<movement_config::Config, anyhow::Error> {
		// Allow Da sync from Height zero
		let local = std::env::var_os("MAYBE_RUN_LOCAL").unwrap_or("false".into());
		if local == "false" {
			config.da_db.allow_sync_from_zero = false;
		} else {
			config.da_db.allow_sync_from_zero = true;
		}
		// update the db path
		let db_path = dot_movement.get_path().join(config.da_db.da_db_path.clone());
		config.da_db.da_db_path = db_path
			.to_str()
			.ok_or(anyhow::anyhow!("Failed to convert db path to string: {:?}", db_path))?
			.to_string();

		Ok(config)
	}
}

impl MovementFullNodeSetupOperations for Local {
	async fn setup(
		&self,
		dot_movement: DotMovement,
		config: movement_config::Config,
	) -> Result<
		(movement_config::Config, tokio::task::JoinHandle<Result<String, anyhow::Error>>),
		anyhow::Error,
	> {
		// Run the DA light node setup
		let (config, join_handle) =
			self.run_da_light_node_setup(dot_movement.clone(), config).await?;

		// run the maptos execution config setup
		let config = self.setup_maptos_execution_config(dot_movement.clone(), config).await?;

		// run the da_db config setup
		let config = self.setup_da_db_config(dot_movement.clone(), config).await?;

		// Placeholder for returning the actual configuration.
		Ok((config, join_handle))
	}
}
