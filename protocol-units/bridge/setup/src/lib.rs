use alloy::node_bindings::AnvilInstance;
use bridge_config::common::bridge::Config;
use dot_movement::DotMovement;
use tokio::process::Child;

pub mod deploy;
pub mod local;

impl Setup {
	pub async fn setup(
		dot_movement: &DotMovement,
		mut config: Config,
	) -> Result<(Config, AnvilInstance, Child), anyhow::Error> {
		// Currently local only
		let (config, anvil, child) = crate::local::setup(dot_movement, config.clone()).await?;
		Ok((config, anvil, child))
	}
}
