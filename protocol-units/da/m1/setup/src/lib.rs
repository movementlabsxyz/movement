pub mod local;
pub mod arabica;
pub mod common;
use m1_da_light_node_util::config::M1DaLightNodeConfig;

pub async fn setup(
	dot_movement: dot_movement::DotMovement,
	mut config: M1DaLightNodeConfig,
) -> Result<M1DaLightNodeConfig, anyhow::Error> {
	let inner_config = match config.m1_da_light_node_config {
		m1_da_light_node_util::config::Config::Local(config) => {
			let local = local::Local::new();
			let local_config = local.setup(dot_movement, config).await?;
			m1_da_light_node_util::config::Config::Local(local_config)
		},
		m1_da_light_node_util::config::Config::Arabica(config) => {
			let arabica = arabica::Arabica::new();
			let arabica_config = arabica.setup(dot_movement, config).await?;
			m1_da_light_node_util::config::Config::Arabica(arabica_config)
		},
	};
	config.m1_da_light_node_config = inner_config;

	Ok(config)
}
