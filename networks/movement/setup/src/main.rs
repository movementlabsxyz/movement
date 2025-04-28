use godfig::{backend::config_file::ConfigFile, Godfig};
use movement_config::Config;
use movement_full_node_setup::local::Local;
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

	let config_file = dot_movement.try_get_or_create_config_file().await?;

	// get a matching godfig object
	let godfig: Godfig<Config, ConfigFile> = Godfig::new(ConfigFile::new(config_file), vec![]);

	// Apply all of the setup steps
	godfig
		.try_transaction_with_result(|config| async move {
			let config = config.unwrap_or_default();

			// set up anvil
			let config = Local::default().setup_da_sequencer(dot_movement, config).await?;

			Ok((Some(config), ()))
		})
		.await?;

	info!("Initial setup complete.");

	Ok(())
}
