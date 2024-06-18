use crate::SuzukaFullNodeSetupOperations;
use dot_movement::DotMovement;
// use mcr_settlement_setup::Setup as _;

// use tracing::debug;

#[derive(Debug, Clone)]
pub struct Local {
	// m1_da_light_node_strategy: m1_da_light_node_setup::local::Local,
	// mcr_settlement_strategy: mcr_settlement_setup::Local,
}

impl Local {
	pub fn new() -> Self {
		Self {
			// m1_da_light_node_strategy: m1_da_light_node_setup::local::Local::new(),
			// mcr_settlement_strategy: Default::default(),
		}
	}

	async fn run_m1_da_light_node_setup(
		&self,
		dot_movement: DotMovement,
		mut config: suzuka_config::Config,
	) -> Result<suzuka_config::Config, anyhow::Error> {

		// Run the m1_da_light_node_setup
		let m1_da_light_node_config = config.m1_da_light_node.clone();

		// Run the m1_da_light_node_setup
		let new_m1_da_light_node_config =
			m1_da_light_node_setup::setup(dot_movement.clone(), m1_da_light_node_config).await?;

		// Update the config with the new m1_da_light_node_config
		config.m1_da_light_node = new_m1_da_light_node_config;
 
		Ok(config)
	}

	async fn setup_maptos_execution_config(
		&self,
		dot_movement: DotMovement,
		mut config: suzuka_config::Config,
	) -> Result<suzuka_config::Config, anyhow::Error> {
		// update the db path
		let chain_id = config.execution_config.maptos_config.chain.maptos_chain_id;
		let db_path = dot_movement
			.get_path()
			.join("maptos")
			.join(chain_id.to_string())
			.join(".maptos");
		config.execution_config.maptos_config.chain.maptos_db_path.replace(db_path);

		Ok(config)
	}

}

impl SuzukaFullNodeSetupOperations for Local {
	async fn setup(
		&self,
		dot_movement: DotMovement,
		config: suzuka_config::Config,
	) -> Result<suzuka_config::Config, anyhow::Error> {
		// Run the m1_da_light_node_setup
		let config = self.run_m1_da_light_node_setup(dot_movement.clone(), config).await?;

		// run the maptos execution config setup
		let config = self.setup_maptos_execution_config(dot_movement.clone(), config).await?;

		// Placeholder for returning the actual configuration.
		Ok(config)
	}
}
