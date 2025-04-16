use movement_da_sequencer_config::DaSequencerConfig;
use movement_da_sequencer_node::whitelist::Whitelist;
use movement_signer::{cryptography::ed25519::Ed25519, Signing};
use movement_signer_loader::{Load, LoadedSigner};

pub async fn setup_movement_node(
	dot_movement: &dot_movement::DotMovement,
	da_sequencer_config: DaSequencerConfig,
	maptos_config: &maptos_execution_util::config::Config,
) -> Result<DaSequencerConfig, anyhow::Error> {
	//update whitelist with node public key.
	let loader: LoadedSigner<Ed25519> =
		maptos_config.chain.maptos_private_key_signer_identifier.load().await?;

	let verifying_key =
		ed25519_dalek::VerifyingKey::from_bytes(&loader.public_key().await?.to_bytes())?;

	let dotmovement_path = dot_movement.get_path().to_path_buf();
	let whitelist_path = dotmovement_path.join(&da_sequencer_config.whitelist_relative_path);
	if whitelist_path.exists() {
		std::fs::remove_file(&whitelist_path)?;
	}
	Whitelist::save(&whitelist_path, &[verifying_key])?;

	tracing::info!("Da Sequencer local setup done.");
	Ok(da_sequencer_config)
}
