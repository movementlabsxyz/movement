pub mod common;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
	/// The ETH connection configuration.
	/// This is mandatory for all possible operations.
	#[serde(default)]
	pub eth: common::eth::EthConfig,

	#[serde(default)]
	pub movement: common::movement::MovementConfig,

	/// Optional testing config
	#[serde(default)]
	pub testing: common::testing::TestingConfig,
}

impl Default for Config {
	fn default() -> Self {
		Config {
			eth: common::eth::EthConfig::default(),
			movement: common::movement::MovementConfig::default(),
			testing: common::testing::TestingConfig::default(),
		}
	}
}

// #[allow(dead_code)]
// async fn testfunction1_mvt() -> Result<(), anyhow::Error> {
// 	let dot_movement = dot_movement::DotMovement::try_from_env()?;
// 	let config_file = dot_movement.try_get_or_create_config_file().await?;

// 	// Get a matching godfig object
// 	let godfig: Godfig<Config, ConfigFile> =
// 		Godfig::new(ConfigFile::new(config_file), vec!["bridge".to_string()]);
// 	let config: Config = godfig.try_wait_for_ready().await?;

// 	// Correct use of println!
// 	println!("{:?}", config);

// 	assert!(true);
// 	Ok(())
// }

// #[allow(dead_code)]
// async fn testfunction2_eth(anvil: AnvilInstance) -> Result<(), anyhow::Error> {
// 	let dot_movement = dot_movement::DotMovement::try_from_env()?;
// 	let config_file = dot_movement.try_get_or_create_config_file().await?;

// 	// Get a matching godfig object
// 	let godfig: Godfig<Config, ConfigFile> =
// 		Godfig::new(ConfigFile::new(config_file), vec!["bridge".to_string()]);
// 	let config: Config = godfig.try_wait_for_ready().await?;

// 	// Correct use of println!
// 	println!("{:?}", config);

// 	assert!(true);
// 	Ok(())
// }

// #[allow(dead_code)]
// async fn testfunction_eth_mvt() -> Result<(), anyhow::Error> {
// 	let dot_movement = dot_movement::DotMovement::try_from_env()?;
// 	let config_file = dot_movement.try_get_or_create_config_file().await?;

// 	// Get a matching godfig object
// 	let godfig: Godfig<Config, ConfigFile> =
// 		Godfig::new(ConfigFile::new(config_file), vec!["bridge".to_string()]);
// 	let config: Config = godfig.try_wait_for_ready().await?;

// 	// Correct use of println!
// 	println!("{:?}", config);

// 	assert!(true);
// 	Ok(())
// }
