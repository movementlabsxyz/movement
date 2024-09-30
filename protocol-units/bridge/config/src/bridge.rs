use godfig::{backend::config_file::ConfigFile, Godfig};
use mcr_settlement_config::Config;
use mcr_settlement_setup::Setup;

#[tokio::test] 
async fn main() -> Result<(), anyhow::Error> {
	let (anvil, child) = setup().await;
	tetsfunction1_mvt();
	tetsfunction2_eth(anvil);
	tetsfunction_eth_mvt();
	child.kill()
}

async setup()-> (AnvilInstance , tokio::process::Child) {
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
		Godfig::new(ConfigFile::new(config_file), vec!["mcr_settlement".to_string()]);

	let anvil = Anvil::new().port(eth_client.rpc_port()).spawn();
start movement child
	// run a godfig transaction to update the file
	godfig
                .try_transaction(|config| async move {
                        println!("Config: {:?}", config);
                        let (config, _) = local.setup(&dot_movement, config).await?;
                        Ok(Some(config))

                })
                .await?;
}

	Ok((anvil, child))
}

async testfunction1_mvt() {
	let dot_movement = dot_movement::DotMovement::try_from_env()?;
	let config_file = dot_movement.try_get_or_create_config_file().await?;

	// get a matching godfig object
	let godfig: Godfig<Config, ConfigFile> =
		Godfig::new(ConfigFile::new(config_file), vec!["mcr_settlement".to_string()]);
	let config: Config = godfig.try_wait_for_ready().await?;
	println(config);
	assert!(true);

}