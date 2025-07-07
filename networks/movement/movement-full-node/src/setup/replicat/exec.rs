use crate::setup::replicat::local;
use godfig::{backend::config_file::ConfigFile, Godfig};
use movement_da_sequencer_config::DaReplicatConfig;
use tracing::info;

pub async fn exec() -> Result<(), anyhow::Error> {
	info!("Starting Movement Full Node Setup");

	// get the config file
	let mut dot_movement = dot_movement::DotMovement::try_from_env()?;

	//Define da-sequencer config path.
	let pathbuff = DaReplicatConfig::get_config_path(&dot_movement);
	dot_movement.set_path(pathbuff);
	// get a matching godfig object
	let config_file = dot_movement.try_get_or_create_config_file().await?;
	let godfig: Godfig<DaReplicatConfig, ConfigFile> =
		Godfig::new(ConfigFile::new(config_file), vec![]);

	// run a godfig transaction to update the file
	godfig
		.try_transaction(|config| async move {
			let mut config = config.unwrap_or(DaReplicatConfig::default());
			let local = std::env::var_os("MAYBE_RUN_LOCAL").unwrap_or("false".into());
			if local == "true" {
				local::setup_movement_replica_node(&dot_movement, &mut config).await?;
			}
			tracing::info!("Da Sequencer Config after local setup: {:?}", config);

			Ok(Some(config))
		})
		.await?;

	Ok(())
}
