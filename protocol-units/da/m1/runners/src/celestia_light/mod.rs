pub mod arabica;
pub mod mocha;
use m1_da_light_node_util::config::M1DaLightNodeConfig;

use crate::Runner;

#[derive(Debug, Clone)]
pub struct CelestiaLight {}

impl Runner for CelestiaLight {
	async fn run(
		&self,
		dot_movement: dot_movement::DotMovement,
		config: M1DaLightNodeConfig,
	) -> Result<(), anyhow::Error> {
		match config.m1_da_light_node_config {
			m1_da_light_node_util::config::Config::Local(config) => {
				Err(anyhow::anyhow!("Local not implemented"))?;
			},
			m1_da_light_node_util::config::Config::Arabica(config) => {
				let arabica = arabica::Arabica::new();
				arabica.run(dot_movement, config).await?;
			},
			m1_da_light_node_util::config::Config::Mocha(config) => {
				let mocha = mocha::Mocha::new();
				mocha.run(dot_movement, config).await?;
			},
		}
		Ok(())
	}
}
