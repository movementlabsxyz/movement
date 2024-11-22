use godfig::{backend::config_file::ConfigFile, Godfig};
use movement_celestia_da_light_node_setup::setup;
use movement_celestia_da_util::config::CelestiaDaLightNodeConfig;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
	use tracing_subscriber::EnvFilter;

	tracing_subscriber::fmt()
		.with_env_filter(
			EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
		)
		.init();

	// get the config file
	let dot_movement = dot_movement::DotMovement::try_from_env()?;
	let mut config_file = dot_movement.try_get_or_create_config_file().await?;

	// get a matching godfig object
	let godfig: Godfig<CelestiaDaLightNodeConfig, ConfigFile> =
		Godfig::new(ConfigFile::new(config_file), vec![]);

	// run a godfig transaction to update the file
	godfig
		.try_transaction(|config| async move {
			println!("Config: {:?}", config);
			match config {
				Some(config) => {
					let config = setup(dot_movement.clone(), config).await?;
					Ok(Some(config))
				}
				None => {
					let config = CelestiaDaLightNodeConfig::default();
					let config = setup(dot_movement.clone(), config).await?;
					Ok(Some(config))
				}
			}
		})
		.await?;

	Ok(())
}
