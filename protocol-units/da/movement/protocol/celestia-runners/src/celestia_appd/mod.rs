pub mod local;
use crate::Runner;
use movement_da_util::config::{CelestiaDaLightNodeConfig, Network};

#[derive(Debug, Clone)]
pub struct CelestiaAppd {}

impl Runner for CelestiaAppd {
	async fn run(
		&self,
		dot_movement: dot_movement::DotMovement,
		config: CelestiaDaLightNodeConfig,
	) -> Result<(), anyhow::Error> {
		let config = config.celestia_da_light_node_config;
		match config.network {
			Network::Local => {
				let local = local::Local::new();
				local.run(dot_movement, config).await?;
				Ok(())
			}
			Network::Arabica => Err(anyhow::anyhow!("Arabica not implemented")),
			Network::Mocha => Err(anyhow::anyhow!("Mocha not implemented")),
			Network::Mainnet => Err(anyhow::anyhow!("Mainnet not implemented")),
		}
	}
}
