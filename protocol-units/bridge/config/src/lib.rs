pub mod common;

use alloy::node_bindings::{AnvilInstance}; 
use dot_movement;
use godfig::{backend::config_file::ConfigFile, Godfig};
use common::bridge::Config;

#[allow(dead_code)]
async fn testfunction1_mvt() -> Result<(), anyhow::Error> {
	let dot_movement = dot_movement::DotMovement::try_from_env()?;
	let config_file = dot_movement.try_get_or_create_config_file().await?;

	// Get a matching godfig object
	let godfig: Godfig<Config, ConfigFile> =
		Godfig::new(ConfigFile::new(config_file), vec!["bridge".to_string()]);
	let config: Config = godfig.try_wait_for_ready().await?;

	// Correct use of println!
	println!("{:?}", config);

	assert!(true);
	Ok(())
}

#[allow(dead_code)]
async fn testfunction2_eth(anvil: AnvilInstance) -> Result<(), anyhow::Error> {
	let dot_movement = dot_movement::DotMovement::try_from_env()?;
	let config_file = dot_movement.try_get_or_create_config_file().await?;
    
	// Get a matching godfig object
	let godfig: Godfig<Config, ConfigFile> =
	    Godfig::new(ConfigFile::new(config_file), vec!["bridge".to_string()]);
	let config: Config = godfig.try_wait_for_ready().await?;
    
	// Correct use of println!
	println!("{:?}", config);
    
	assert!(true);
	Ok(())
}

#[allow(dead_code)]
async fn testfunction_eth_mvt() -> Result<(), anyhow::Error> {
	let dot_movement = dot_movement::DotMovement::try_from_env()?;
	let config_file = dot_movement.try_get_or_create_config_file().await?;
    
	// Get a matching godfig object
	let godfig: Godfig<Config, ConfigFile> =
	    Godfig::new(ConfigFile::new(config_file), vec!["bridge".to_string()]);
	let config: Config = godfig.try_wait_for_ready().await?;
    
	// Correct use of println!
	println!("{:?}", config);
    
	assert!(true);
	Ok(())
}
