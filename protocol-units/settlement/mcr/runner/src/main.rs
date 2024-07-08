use mcr_settlement_config::Config;
use godfig::{
	Godfig,
	backend::config_file::ConfigFile
};
use mcr_settlement_setup::{Setup, Local, deploy_remote::DeployRemote};

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
	let godfig : Godfig<Config, ConfigFile> = Godfig::new(ConfigFile::new(config_file), vec![
        "mcr_settlement".to_string(),
    ]);

	// Apply all of the setup steps
	let anvil_join_handle = godfig.try_transaction_with_result(|config| async move {

		tracing::info!("Config: {:?}", config);
		let config = config.unwrap_or_default();
		tracing::info!("Config: {:?}", config);

		let (config, anvil_join_handle) = match config {
			Config::Local(config) => {
				let local = Local::default();
				let (config, anvil_join_handle) = local.setup(&dot_movement, Config::Local(config)).await?;
				(config, anvil_join_handle)
			},
			Config::DeployRemote(config) => {
				let remote_deploy = DeployRemote::default();
				let (config, anvil_join_handle) = remote_deploy.setup(&dot_movement, Config::Local(config)).await?;
				(config, anvil_join_handle)
			},
		};
	
		Ok((Some(config), anvil_join_handle))

	}).await?;

	// wait for anvil to finish
	let _ = anvil_join_handle.await?;

	Ok(())
}
