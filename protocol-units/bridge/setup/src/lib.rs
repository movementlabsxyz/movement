use alloy::node_bindings::AnvilInstance;
use bridge_config::Config;

pub mod deploy;
pub mod local;

pub async fn process_compose_setup(config: Config) -> Result<Config, anyhow::Error> {
	// Currently local only
	tracing::info!("Bridge process_compose_setup");

	//Deploy locally
	let config = crate::deploy::setup(config).await?;
	Ok(config)
}

pub async fn test_eth_setup(mut config: Config) -> Result<Config, anyhow::Error> {
	//let anvil = local::setup_eth(&mut config.eth, &mut config.testing);
	//Define the timelock to 15s for the test
	config.eth.time_lock_secs = 15;
	//Deploy locally
	crate::deploy::setup_local_ethereum(&mut config).await?;
	Ok(config)
}

pub async fn test_mvt_setup(mut config: Config) -> Result<Config, anyhow::Error> {
	//Set working dir to project path becasue movement cli need it to be set.
	let dot_movement = dot_movement::DotMovement::try_from_env()?;
	let root_path = dot_movement.get_path().parent().unwrap_or(std::path::Path::new("/"));
	std::env::set_current_dir(&root_path)?;

	//	let movement_task = local::setup_movement_node(&mut config.movement).await?;
	deploy::deploy_local_movement_node(&mut config.movement)?;
	Ok(config)
}
