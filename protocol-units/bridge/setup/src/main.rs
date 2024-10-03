mod local;

use bridge_config::Config;
use godfig::{backend::config_file::ConfigFile, Godfig};

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
	let config_file = dot_movement.try_get_or_create_config_file().await?;

	// get a matching godfig object
	let godfig: Godfig<Config, ConfigFile> =
		Godfig::new(ConfigFile::new(config_file), vec!["bridge".to_string()]);

	// run a godfig transaction to update the file
	godfig
		.try_transaction(|config| async move {
			tracing::info!("Bridge Default Config: {:?}", config);
			let config = config.unwrap_or(Config::default());

			let (config, _anvil) = bridge_setup::process_compose_setup(config).await?;
			let config = bridge_setup::deploy::setup(config).await?;
			tracing::info!("Bridge Config: {:?}", config);

			Ok(Some(config))
		})
		.await?;

	println!("Config after update:",);
	//Wait indefinitely to keep the Anvil process alive.
	let join_handle: tokio::task::JoinHandle<()> =
		tokio::spawn(async { std::future::pending().await });
	let _ = join_handle.await;
	Ok(())
}
