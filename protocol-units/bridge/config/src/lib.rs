pub mod common;

use alloy::node_bindings::{Anvil, AnvilInstance}; 
use dot_movement;
use godfig::{backend::config_file::ConfigFile, Godfig};
use mcr_settlement_config::Config;
use mcr_settlement_setup::local::Local;
use tokio::process::Child;
use tracing_subscriber::EnvFilter;
use tracing_subscriber;

#[tokio::test]
async fn run_all_tests() -> Result<(), anyhow::Error> {
    let (anvil, child) = setup().await?;
    testfunction1_mvt().await?;
    testfunction2_eth(anvil).await?;
    testfunction_eth_mvt().await?;
    child.kill().await?;
    Ok(())
}

async fn setup() -> Result<(AnvilInstance, Child), anyhow::Error> {
	// Initialize tracing
	tracing_subscriber::fmt()
		.with_env_filter(
		EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
		)
		.init();

	// Get the config file
	let dot_movement = dot_movement::DotMovement::try_from_env()?;
	let config_file = dot_movement.try_get_or_create_config_file().await?;

	// Get a matching godfig object
	let godfig: Godfig<Config, ConfigFile> =
		Godfig::new(ConfigFile::new(config_file), vec!["mcr_settlement".to_string()]);

	// Start Anvil and movement processes
	let anvil = Anvil::new().port(eth_client.rpc_port()).spawn();
	let child = start_movement_child().await?; // Assuming this is a function to start movement child

	// Run a godfig transaction to update the file
	godfig
		.try_transaction(|config| async move {
		println!("Config: {:?}", config);
		let (config, _) = Local::setup(&self, &dot_movement, config).await?;
		Ok(Some(config))
		})
		.await?;

	Ok((anvil, child))
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

async fn testfunction2_eth(anvil) -> Result<(), anyhow::Error> {
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
