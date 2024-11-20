pub mod local;
use movement_celestia_da_util::config::CelestiaDaLightNodeConfig;

use crate::Runner;

#[derive(Debug, Clone)]
pub struct CelestiaBridge {}

impl Runner for CelestiaBridge {
	async fn run(
		&self,
		dot_movement: dot_movement::DotMovement,
		config: CelestiaDaLightNodeConfig,
	) -> Result<(), anyhow::Error> {
		match config.celestia_da_light_node_config {
			movement_celestia_da_util::config::Config::Local(config) => {
				let local = local::Local::new();
				local.run(dot_movement, config).await?;
			}
			movement_celestia_da_util::config::Config::Arabica(_config) => {
				Err(anyhow::anyhow!("Arabica not implemented"))?;
			}
			movement_celestia_da_util::config::Config::Mocha(_config) => {
				Err(anyhow::anyhow!("Mocha not implemented"))?;
			}
		}
		Ok(())
	}
}
