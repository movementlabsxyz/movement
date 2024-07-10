use mcr_settlement_setup::{Setup, Local};
use mcr_settlement_config::Config;
use godfig::{
	Godfig,
	backend::config_file::ConfigFile
};

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
	let godfig : Godfig<Config, ConfigFile> = Godfig::new(ConfigFile::new(config_file), vec![
        "mcr_settlement".to_string(),
    ]);

	// run a godfig transaction to update the file
	godfig.try_transaction(|config| async move {
		println!("Config: {:?}", config);
        let local = Local::default();
		match config {
			Some(config) => {
				let (config, _) = local.setup(&dot_movement, config).await?;
				Ok(Some(config))
			},
			None => {
				let config = Config::default();
				let (config, _ ) = local.setup(&dot_movement, config).await?;
				Ok(Some(config))
			}
		}
	}).await?;

	Ok(())
}
