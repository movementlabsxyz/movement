use alloy::node_bindings::AnvilInstance;
use dot_movement::DotMovement;
use bridge_config::common::bridge::Config;
use tokio::process::Child;

pub mod deploy;
pub mod local;

#[derive(Debug, Clone, Default)]
pub struct Setup {
	pub local: local::Local,
	pub deploy: deploy::Deploy,
}

impl Setup {
	pub fn new() -> Self {
	Self {
		local: local::Local::new(),
		deploy: deploy::Deploy::new(),
	}
	}

	pub async fn setup(
	&self,
	dot_movement: &DotMovement,
	mut config: Config,
	use_local: bool, 
	) -> Result<(Config, AnvilInstance, Child), anyhow::Error> {
		if use_local {
			let (config, anvil, child) = self.local.setup(dot_movement, config.clone()).await?;
			Ok((config, anvil, child))
		} else {
			let (config, anvil, child) = self.deploy.setup(dot_movement, config.clone()).await?;
			Ok((config, anvil, child))
		}
	}
}
