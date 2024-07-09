use m1_da_light_node_runners::{celestia_bridge::CelestiaBridge, Runner};
use godfig::{
	Godfig,
	backend::config_file::ConfigFile
};
use m1_da_light_node_util::M1DaLightNodeConfig;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
	use tracing_subscriber::EnvFilter;

	tracing_subscriber::fmt()
		.with_env_filter(
			EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
		)
		.init();

	let dot_movement = dot_movement::DotMovement::try_from_env()?;
	let mut config_file = dot_movement.try_get_or_create_config_file().await?;

	// get a matching godfig object
	let godfig : Godfig<M1DaLightNodeConfig, ConfigFile> = Godfig::new(ConfigFile::new(config_file), vec![]);
	let config = godfig.try_wait_for_ready().await?;

	let celestia_bridge = CelestiaBridge {};
	celestia_bridge.run(dot_movement, config).await?;

	Ok(())
}
