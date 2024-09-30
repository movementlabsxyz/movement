pub mod common;

use alloy::node_bindings::{Anvil, AnvilInstance}; 
use bridge_service::chains::ethereum::client::{Config as EthConfig, EthClient};
use bridge_service::chains::movement::client::{MovementClient, Config as MovementConfig};
use dot_movement;
use godfig::{backend::config_file::ConfigFile, Godfig};
use mcr_settlement_config::Config;
use tokio::process::Child;
use tracing_subscriber::EnvFilter;
use tracing_subscriber;
use common::bridge::Config as BridgeConfig;


#[tokio::test]
async fn run_all_tests() -> Result<(), anyhow::Error> {
    let (anvil, mut child) = setup().await?;
    testfunction1_mvt().await?;
    testfunction2_eth(anvil).await?;
    testfunction_eth_mvt().await?;
    child.kill().await?;
    Ok(())
}

async fn testfunction1_mvt() -> Result<(), anyhow::Error> {
	let dot_movement = dot_movement::DotMovement::try_from_env()?;
	let config_file = dot_movement.try_get_or_create_config_file().await?;

	// Get a matching godfig object
	let godfig: Godfig<Config, ConfigFile> =
		Godfig::new(ConfigFile::new(config_file), vec!["mcr_settlement".to_string()]);
	let config: Config = godfig.try_wait_for_ready().await?;

	// Correct use of println!
	println!("{:?}", config);

	assert!(true);
	Ok(())
}

async fn testfunction2_eth(anvil: AnvilInstance) -> Result<(), anyhow::Error> {
	let dot_movement = dot_movement::DotMovement::try_from_env()?;
	let config_file = dot_movement.try_get_or_create_config_file().await?;
    
	// Get a matching godfig object
	let godfig: Godfig<Config, ConfigFile> =
	    Godfig::new(ConfigFile::new(config_file), vec!["mcr_settlement".to_string()]);
	let config: Config = godfig.try_wait_for_ready().await?;
    
	// Correct use of println!
	println!("{:?}", config);
    
	assert!(true);
	Ok(())
}

async fn testfunction_eth_mvt() -> Result<(), anyhow::Error> {
	let dot_movement = dot_movement::DotMovement::try_from_env()?;
	let config_file = dot_movement.try_get_or_create_config_file().await?;
    
	// Get a matching godfig object
	let godfig: Godfig<Config, ConfigFile> =
	    Godfig::new(ConfigFile::new(config_file), vec!["mcr_settlement".to_string()]);
	let config: Config = godfig.try_wait_for_ready().await?;
    
	// Correct use of println!
	println!("{:?}", config);
    
	assert!(true);
	Ok(())
}
