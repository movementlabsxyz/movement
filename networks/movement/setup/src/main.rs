use godfig::{backend::config_file::ConfigFile, Godfig};
use movement_config::Config;
use movement_full_node_setup::local::Local;
use movement_full_node_setup::migrate::migrate_v0_4_0;
use std::path::Path;
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
	use tracing_subscriber::EnvFilter;

	tracing_subscriber::fmt()
		.with_env_filter(
			EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
		)
		.init();

	info!("Starting Movement Full Node Setup");

	// get the config file
	let dot_movement = dot_movement::DotMovement::try_from_env()?;

	// Detect if the config file exist.
	// If yes do the migrate
	// Otherwise create it
	let config_path = dot_movement.get_config_json_path();
	if Path::new(&config_path).exists() {
		info!("Configuration file found, start migration.");
		migrate_v0_4_0(dot_movement).await?;
	} else {
		info!("No Configuration file found, create a new one.");
		let config_file = dot_movement.try_get_or_create_config_file().await?;

		// get a matching godfig object
		let godfig: Godfig<Config, ConfigFile> = Godfig::new(ConfigFile::new(config_file), vec![]);

		// Apply all of the setup steps
		godfig
			.try_transaction_with_result(|config| async move {
				let config = config.unwrap_or_default();

				let config = Local::default().setup_da_sequencer(dot_movement, config).await?;

				Ok((Some(config), ()))
			})
			.await?;
		info!("Initial setup complete.");
	}

	Ok(())
}
