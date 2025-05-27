use dot_movement::DotMovement;
use mcr_settlement_config::{common, Config};

/// The local setup strategy for MCR settlement
#[derive(Debug, Clone)]
pub struct Deploy {}

impl Deploy {
	/// Instantiates the local setup strategy with ports on localhost
	/// to configure for Ethernet RPC and WebSocket client access.
	pub fn new() -> Self {
		Self {}
	}
}

impl Default for Deploy {
	fn default() -> Self {
		Deploy::new()
	}
}

impl Deploy {
	pub async fn setup(
		&self,
		_dot_movement: &DotMovement,
		mut config: Config,
		deploy: &common::deploy::Config,
	) -> Result<Config, anyhow::Error> {
		// enforce config.deploy = deploy
		Ok(config)
	}
}
