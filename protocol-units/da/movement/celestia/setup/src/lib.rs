pub mod arabica;
pub mod common;
pub mod local;
pub mod mocha;
use movement_celestia_da_util::config::CelestiaDaLightNodeConfig;

pub async fn setup(
	dot_movement: dot_movement::DotMovement,
	mut config: CelestiaDaLightNodeConfig,
) -> Result<CelestiaDaLightNodeConfig, anyhow::Error> {
	let inner_config = match config.celestia_da_light_node_config {
		movement_celestia_da_util::config::Config::Local(config) => {
			let local = local::Local::new();
			let local_config = local.setup(dot_movement, config).await?;
			movement_celestia_da_util::config::Config::Local(local_config)
		}
		movement_celestia_da_util::config::Config::Arabica(config) => {
			let arabica = arabica::Arabica::new();
			let arabica_config = arabica.setup(dot_movement, config).await?;
			movement_celestia_da_util::config::Config::Arabica(arabica_config)
		}
		movement_celestia_da_util::config::Config::Mocha(config) => {
			let mocha = mocha::Mocha::new();
			let mocha_config = mocha.setup(dot_movement, config).await?;
			movement_celestia_da_util::config::Config::Mocha(mocha_config)
		}
	};
	config.celestia_da_light_node_config = inner_config;

	Ok(config)
}
