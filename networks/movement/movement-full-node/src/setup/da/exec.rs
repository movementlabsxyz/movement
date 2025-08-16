use super::local;
use godfig::{backend::config_file::ConfigFile, Godfig};
use movement_da_sequencer_config::DaSequencerConfig;

pub async fn exec() -> Result<(), anyhow::Error> {
	// get the config file
	let mut dot_movement = dot_movement::DotMovement::try_from_env()?;

	// Load Maptos config
	let maptos_config = {
		let config_file = dot_movement.try_get_or_create_config_file().await?;
		let godfig: Godfig<maptos_execution_util::config::Config, ConfigFile> =
			Godfig::new(ConfigFile::new(config_file), vec!["maptos_config".to_string()]);
		godfig.try_wait_for_ready().await
	}?;

	//Define da-sequencer config path.
	let pathbuff = DaSequencerConfig::get_config_path(&dot_movement);
	dot_movement.set_path(pathbuff);
	// get a matching godfig object
	let config_file = dot_movement.try_get_or_create_config_file().await?;
	let godfig: Godfig<DaSequencerConfig, ConfigFile> =
		Godfig::new(ConfigFile::new(config_file), vec![]);

	// run a godfig transaction to update the file
	godfig
		.try_transaction(|config| async move {
			let config = config.unwrap_or(DaSequencerConfig::default());
			let config = local::setup_movement_node(&dot_movement, config, &maptos_config).await?;
			tracing::info!("Da Sequencer Config after local setup: {:?}", config);

			Ok(Some(config))
		})
		.await?;

	println!("Da Sequencer setup done.",);
	Ok(())
}
