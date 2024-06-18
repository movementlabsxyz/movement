use crate::MonzaFullNodeSetupOperations;
use dot_movement::DotMovement;
use m1_da_light_node_setup::M1DaLightNodeSetupOperations;
use monza_config::Config;
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
		dot_movement: &DotMovement,
		mut config: Config,
	) -> Result<Config, anyhow::Error> {
		// Get the m1_da_light_node_config from the suzuka config
		let m1_da_light_node_config = config.execution_config.light_node_config.clone();

		// Run the m1_da_light_node_setup
		info!("Running m1_da_light_node_setup");
		let m1_da_light_node_config = self
			.m1_da_light_node_strategy
			.setup(dot_movement, m1_da_light_node_config)
			.await?;

		// Modify the suzuka config accordingly
		config.execution_config.light_node_config = m1_da_light_node_config;

		Ok(config)
	}
}

impl MonzaFullNodeSetupOperations for Local {
	async fn setup(
		&self,
		dot_movement: &DotMovement,
		config: Config,
	) -> Result<Config, anyhow::Error> {
		// Run the m1_da_light_node_setup
		let config = self.run_m1_da_light_node_setup(dot_movement, config).await?;

		// Placeholder for returning the actual configuration.
		Ok(config)
	}
}
