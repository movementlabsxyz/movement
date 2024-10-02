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

pub async fn test_eth_setup() -> Result<(Config, AnvilInstance), anyhow::Error> {
	let mut config = Config::default();
	let anvil = local::setup_eth(&mut config.eth, &mut config.testing);
	//Deploy locally
	crate::deploy::setup_local_ethereum(&mut config.eth).await?;
	Ok((config, anvil))
}

pub async fn test_mvt_setup() -> Result<(Config, tokio::process::Child), anyhow::Error> {
	let mut config = Config::default();
	let movement_task = local::setup_movement_node(&mut config.movement).await?;
	local::init_local_movement_node(&mut config.movement)?;
	Ok((config, movement_task))
}
