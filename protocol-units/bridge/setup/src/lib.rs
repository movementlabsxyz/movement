use alloy::node_bindings::AnvilInstance;
use bridge_config::Config;

pub mod deploy;
pub mod local;

pub async fn process_compose_setup(
	config: Config,
) -> Result<(Config, AnvilInstance), anyhow::Error> {
	// Currently local only
	tracing::info!("Bridge process_compose_setup");
	let (config, anvil) = crate::local::setup(config).await?;

	//Deploy locally
	let config = crate::deploy::setup(config).await?;
	Ok((config, anvil))
}

pub async fn test_setup(config: Config) -> Result<(Config, AnvilInstance), anyhow::Error> {
	// Currently local only
	let (config, anvil) = crate::local::setup(config).await?;

	//Deploy locally
	let config = crate::deploy::setup(config).await?;
	Ok((config, anvil))
}
