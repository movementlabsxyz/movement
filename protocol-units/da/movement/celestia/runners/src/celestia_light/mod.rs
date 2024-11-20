pub mod arabica;
pub mod mocha;
use movement_celestia_da_util::config::CelestiaDaLightNodeConfig;

use crate::Runner;

#[derive(Debug, Clone)]
pub struct CelestiaLight {}

impl Runner for CelestiaLight {
	async fn run(
		&self,
		dot_movement: dot_movement::DotMovement,
		config: CelestiaDaLightNodeConfig,
	) -> Result<(), anyhow::Error> {
		match config.celestia_da_light_node_config {
			movement_celestia_da_util::config::Config::Local(_config) => {
				Err(anyhow::anyhow!("Local not implemented"))?;
			}
			movement_celestia_da_util::config::Config::Arabica(config) => {
				let arabica = arabica::Arabica::new();
				arabica.run(dot_movement, config).await?;
			}
			movement_celestia_da_util::config::Config::Mocha(config) => {
				let mocha = mocha::Mocha::new();
				mocha.run(dot_movement, config).await?;
			}
		}
		Ok(())
	}
}
