#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
	use tracing_subscriber::EnvFilter;

	tracing_subscriber::fmt()
		.with_env_filter(
			EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
		)
		.init();

	let dot_movement = dot_movement::DotMovement::try_from_env()?;
	let replicat_config_path = get_config_path(&dot_movement);
	tracing::info!("replicat_config_path:{replicat_config_path:?}");
	movement_da_sequencer_setup::setup(replicat_config_path).await?;

	let bind = std::env::var_os("MOVEMENT_DA_SEQUENCER_GRPC_LISTEN_ADDRESS");
	tracing::info!("MOVEMENT_DA_SEQUENCER_GRPC_LISTEN_ADDRESS:{bind:?}");

	// let local = std::env::var_os("MAYBE_RUN_LOCAL").unwrap_or("false".into());
	// if local == "true" {
	// 	//update replicat port to avoid conflict with da sequecner in local.

	// }
	Ok(())
}

pub const DA_REPLICAT_DIR: &str = "da-replicat";
pub fn get_config_path(dot_movement: &dot_movement::DotMovement) -> std::path::PathBuf {
	let mut pathbuff = std::path::PathBuf::from(dot_movement.get_path());
	pathbuff.push(DA_REPLICAT_DIR);
	pathbuff
}
