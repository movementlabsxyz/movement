pub mod arabica;
pub mod common;
pub mod local;
pub mod mainnet;
pub mod mocha;
use movement_da_util::config::{CelestiaDaLightNodeConfig, Network};

pub async fn setup(
	dot_movement: dot_movement::DotMovement,
	mut config: CelestiaDaLightNodeConfig,
) -> Result<CelestiaDaLightNodeConfig, anyhow::Error> {
	let inner_config = config.celestia_da_light_node_config;
	let inner_config = match inner_config.network {
		Network::Local => {
			let local = local::Local::new();
			local.setup(dot_movement, inner_config).await?
		}
		Network::Arabica => {
			let arabica = arabica::Arabica::new();
			arabica.setup(dot_movement, inner_config).await?
		}
		Network::Mocha => {
			let mocha = mocha::Mocha::new();
			mocha.setup(dot_movement, inner_config).await?
		}
		Network::Mainnet => {
			let mainnet = mainnet::Mainnet::new();
			mainnet.setup(dot_movement, inner_config).await?
		}
	};
	config.celestia_da_light_node_config = inner_config;

	Ok(config)
}
