pub mod arabica;
pub mod mainnet;
pub mod mocha;

use movement_da_util::config::{CelestiaDaLightNodeConfig, Network};

use crate::Runner;

#[derive(Debug, Clone)]
pub struct CelestiaLight {}

impl Runner for CelestiaLight {
	async fn run(
		&self,
		dot_movement: dot_movement::DotMovement,
		config: CelestiaDaLightNodeConfig,
	) -> Result<(), anyhow::Error> {
		let config = config.celestia_da_light_node_config;
		match config.network {
			Network::Local => {
				Err(anyhow::anyhow!("Local not implemented"))?;
			}
			Network::Arabica => {
				let arabica = arabica::Arabica::new();
				arabica.run(dot_movement, config).await?;
			}
			Network::Mocha => {
				let mocha = mocha::Mocha::new();
				mocha.run(dot_movement, config).await?;
			}
			Network::Mainnet => {
				let mainnet = mainnet::Mainnet::new();
				mainnet.run(dot_movement, config).await?;
			}
		}
		Ok(())
	}
}
