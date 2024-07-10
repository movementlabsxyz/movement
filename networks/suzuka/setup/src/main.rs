use suzuka_full_node_setup::{local::Local, SuzukaFullNodeSetupOperations};
use godfig::{
	Godfig,
	backend::config_file::ConfigFile
};
use suzuka_config::Config;

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
	let godfig : Godfig<Config, ConfigFile> = Godfig::new(ConfigFile::new(config_file), vec![]);

	// Apply all of the setup steps
	let anvil_join_handle = godfig.try_transaction_with_result(|config| async move {

		tracing::info!("Config: {:?}", config);
		let config = config.unwrap_or_default();
		tracing::info!("Config: {:?}", config);

		let (config, anvil_join_handle) = Local::default().setup(dot_movement, config).await?;
	
		Ok((Some(config), anvil_join_handle))

	}).await?;

	let (tx, rx) = tokio::sync::oneshot::channel::<u8>();

	// Use tokio::select! to wait for either the handle or a cancellation signal
	tokio::select! {
		_ = anvil_join_handle => {
			tracing::info!("Anvil task finished.");
		}
		_ = rx => {
			tracing::info!("Cancellation received, killing anvil task.");
			// Do any necessary cleanup here
		}
	}

	// Ensure the cancellation sender is dropped to clean up properly
	drop(tx);

	Ok(())
}
