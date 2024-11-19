pub mod local;
use crate::Runner;
use movement_celestia_da_util::config::CelestiaDaLightNodeConfig;

#[derive(Debug, Clone)]
pub struct CelestiaAppd {}

impl Runner for CelestiaAppd {
	async fn run(
		&self,
		dot_movement: dot_movement::DotMovement,
		config: CelestiaDaLightNodeConfig,
	) -> Result<(), anyhow::Error> {
		match config.celestia_da_light_node_config {
			movement_celestia_da_util::config::Config::Local(config) => {
				let local = local::Local::new();
				local.run(dot_movement, config).await?;
				Ok(())
			}
			movement_celestia_da_util::config::Config::Arabica(config) => {
				Err(anyhow::anyhow!("Arabica not implemented"))
			}
			movement_celestia_da_util::config::Config::Mocha(config) => {
				Err(anyhow::anyhow!("Mocha not implemented"))
			}
		}
	}
}
