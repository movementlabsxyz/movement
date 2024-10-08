use suzuka_full_node::manager::Manager;

use std::process::ExitCode;

#[tokio::main]
async fn main() -> Result<ExitCode, anyhow::Error> {
	let tracing_config = movement_tracing::Config::from_env()?;
	movement_tracing::init_tracing_subscriber(tracing_config)?;

	// get the config file
	let dot_movement = dot_movement::DotMovement::try_from_env()?;
	let config_file = dot_movement.try_get_or_create_config_file().await?;

	let manager = Manager::new(config_file).await?;
	manager.try_run().await?;

	Ok(ExitCode::SUCCESS)
}
