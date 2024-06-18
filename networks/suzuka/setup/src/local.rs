use crate::SuzukaFullNodeSetupOperations;
use dot_movement::DotMovement;
use m1_da_light_node_setup::M1DaLightNodeSetupOperations;
use mcr_settlement_setup::Setup as _;

use tracing::debug;

#[derive(Debug, Clone)]
pub struct Local {
	m1_da_light_node_strategy: m1_da_light_node_setup::local::Local,
	mcr_settlement_strategy: mcr_settlement_setup::Local,
}

impl Local {
	pub fn new() -> Self {
		Self {
			m1_da_light_node_strategy: m1_da_light_node_setup::local::Local::new(),
			mcr_settlement_strategy: Default::default(),
		}
	}

	async fn run_m1_da_light_node_setup(
		&self,
		dot_movement: DotMovement,
		mut config: suzuka_config::Config,
	) -> Result<suzuka_config::Config, anyhow::Error> {
		// Get the m1_da_light_node_config from the suzuka config
		let m1_da_light_node_config = config.execution_config.light_node_config.clone();

		// Run the m1_da_light_node_setup
		debug!("Running m1_da_light_node_setup");
		let m1_da_light_node_config = self
			.m1_da_light_node_strategy
			.setup(dot_movement.clone(), m1_da_light_node_config)
			.await?;

		// Modify the suzuka config accordingly
		config.execution_config.light_node_config = m1_da_light_node_config;

		debug!("Running mcr_settlement_setup");
		let mcr_settlement_config = config.mcr.clone();
		config.mcr =
			self.mcr_settlement_strategy.setup(&dot_movement, mcr_settlement_config).await?;

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

		// Placeholder for returning the actual configuration.
		Ok(config)
	}
}
