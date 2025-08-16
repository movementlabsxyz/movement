use godfig::backend::config_file::ConfigFile;
use godfig::Godfig;
use movement_da_sequencer_config::DaReplicatConfig;
use movement_da_sequencer_node::whitelist::Whitelist;
use movement_signer::{cryptography::ed25519::Ed25519, Signing};
use movement_signer_loader::{Load, LoadedSigner};

pub async fn setup_movement_replica_node(
	replicat_dot_movement: &dot_movement::DotMovement,
	da_replicat_config: &mut DaReplicatConfig,
) -> Result<(), anyhow::Error> {
	//update whitelist with node public key.
	// Load Maptos config
	let maptos_config = {
		let dot_movement = dot_movement::DotMovement::try_from_env()?;
		let config_file = dot_movement.try_get_or_create_config_file().await?;
		let godfig: Godfig<maptos_execution_util::config::Config, ConfigFile> =
			Godfig::new(ConfigFile::new(config_file), vec!["maptos_config".to_string()]);
		godfig.try_wait_for_ready().await
	}?;

	let loader: LoadedSigner<Ed25519> =
		maptos_config.da_sequencer.batch_signer_identifier.load().await?;

	let verifying_key =
		ed25519_dalek::VerifyingKey::from_bytes(&loader.public_key().await?.to_bytes())?;

	let dotmovement_path = replicat_dot_movement.get_path().to_path_buf();
	let whitelist_path =
		dotmovement_path.join(&da_replicat_config.da_sequencer.whitelist_relative_path);
	if whitelist_path.exists() {
		std::fs::remove_file(&whitelist_path)?;
	}
	Whitelist::save(&whitelist_path, &[verifying_key])?;

	// Register the full node has main node for state propagation.
	let pk_str = hex::encode(verifying_key.to_bytes());

	da_replicat_config.da_sequencer.main_node_verifying_key = Some(pk_str);

	//set the same batch identifier as the fullnode
	da_replicat_config.da_client.batch_signer_identifier =
		maptos_config.da_sequencer.batch_signer_identifier;

	tracing::info!("Da Sequencer local setup done.");
	Ok(())
}
