use suzuka_full_node_setup::{local::Local, SuzukaFullNodeSetupOperations};

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
	use tracing_subscriber::EnvFilter;

	tracing_subscriber::fmt()
		.with_env_filter(
			EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
		)
		.init();

	let dot_movement = dot_movement::DotMovement::try_from_env()?;
	let config = dot_movement
		.try_get_config_from_json::<suzuka_config::Config>()
		.unwrap_or_default();

	let local = Local::new();
	let config = local.setup(dot_movement.clone(), config).await?;

	dot_movement.try_write_config_to_json(&config)?;

	Ok(())
}
