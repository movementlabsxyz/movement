use anyhow::Context;
use suzuka_full_node_setup::{local::Local, SuzukaFullNodeSetupOperations};
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
	use tracing_subscriber::EnvFilter;

	tracing_subscriber::fmt()
		.with_env_filter(
			EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
		)
		.init();

	let dot_movement = dot_movement::DotMovement::try_from_env()?;
	let config = dot_movement.try_get_config_from_json::<suzuka_config::Config>()?;

	let local = Local::new();
	let config = local.setup(dot_movement, config).await?;

	info!("Writing the updated config {:#?} to file: {:?}", config, path);
	config
		.try_write_to_toml_file(&path)
		.context("Failed to write the updated config")?;

	Ok(())
}
