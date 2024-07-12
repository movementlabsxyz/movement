pub mod local;
use crate::Runner;
use m1_da_light_node_util::config::M1DaLightNodeConfig;

#[derive(Debug, Clone)]
pub struct CelestiaAppd {}

impl Runner for CelestiaAppd {
	async fn run(
		&self,
		dot_movement: dot_movement::DotMovement,
		config: M1DaLightNodeConfig,
	) -> Result<(), anyhow::Error> {
		match config.m1_da_light_node_config {
			m1_da_light_node_util::config::Config::Local(config) => {
				let local = local::Local::new();
				local.run(dot_movement, config).await?;
				Ok(())
			},
			m1_da_light_node_util::config::Config::Arabica(config) => {
				Err(anyhow::anyhow!("Arabica not implemented"))
			},
		}
	}
}
