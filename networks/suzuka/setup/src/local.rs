use crate::SuzukaFullNodeSetupOperations;
use dot_movement::DotMovement;
use m1_da_light_node_setup::M1DaLightNodeSetupOperations;
use tracing::info;

#[derive(Debug, Clone)]
pub struct Local {
	m1_da_light_node_strategy: m1_da_light_node_setup::local::Local,
}

impl Local {
	pub fn new() -> Self {
		Self { m1_da_light_node_strategy: m1_da_light_node_setup::local::Local::new() }
	}

	async fn run_m1_da_light_node_setup(
		&self,
		dot_movement: DotMovement,
		mut config: suzuka_config::Config,
	) -> Result<suzuka_config::Config, anyhow::Error> {
		// Get the m1_da_light_node_config from the suzuka config
		let m1_da_light_node_config = config.execution_config.light_node_config.clone();

		// Run the m1_da_light_node_setup
		info!("Running m1_da_light_node_setup");
		let m1_da_light_node_config = self
			.m1_da_light_node_strategy
			.setup(dot_movement.clone(), m1_da_light_node_config)
			.await?;

		// Modify the suzuka config accordingly
		config.execution_config.light_node_config = m1_da_light_node_config;
		info!("Updated M1 DA Light Node Config in Suzuka Config");

		Ok(config)
	}

	async fn setup_maptos_execution_config(
		&self,
		dot_movement: DotMovement,
		mut config: suzuka_config::Config,
	) -> Result<suzuka_config::Config, anyhow::Error> {
		let mut maptos_execution_config = config.execution_config.try_aptos_config()?;

		// update the db path
		let chain_id = maptos_execution_config.try_chain_id()?;
		let db_path = dot_movement
			.get_path()
			.join("maptos")
			.join(chain_id.to_string())
			.join(".maptos");
		maptos_execution_config.aptos_db_path.replace(db_path);

		// update the maptos execution config
		config.execution_config.aptos_config = Some(maptos_execution_config);

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
		info!("SuzuakFullNodeSetup: Running M1 DA Light Node Setup");
		let config = self.run_m1_da_light_node_setup(dot_movement.clone(), config).await?;

		// setup the maptos execution config
		info!("SuzuakFullNodeSetup: Setting up Maptos Execution Config");
		let config = self.setup_maptos_execution_config(dot_movement.clone(), config).await?;

		// Placeholder for returning the actual configuration.
		Ok(config)
	}
}
