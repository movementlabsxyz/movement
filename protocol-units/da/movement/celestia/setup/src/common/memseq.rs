use dot_movement::DotMovement;
use movement_celestia_da_util::config::local::Config;

pub fn initialize_memseq_config(
	dot_movement: DotMovement,
	mut config: Config,
) -> Result<Config, anyhow::Error> {
	// use the dot movement path to set up the memseq database path
	let dot_movement_path = dot_movement.get_path();

	// use the chain id from the celestia config to set up the memseq database path
	let chain_id = config.appd.celestia_chain_id.clone();

	// update the memseq database path with the chain id
	let path = dot_movement_path
		.join("memseq")
		.join(chain_id.clone())
		.join(".memseq")
		.to_str()
		.ok_or(anyhow::anyhow!("Failed to convert path to string."))?
		.to_string();
	config.memseq.sequencer_chain_id = Some(chain_id.clone());
	config.memseq.sequencer_database_path = Some(path.clone());

	Ok(config)
}
