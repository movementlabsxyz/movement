use suzuka_full_node::manager::Manager;

use std::process::ExitCode;

#[tokio::main]
async fn main() -> Result<ExitCode, anyhow::Error> {
	movement_tracing::init_tracing_subscriber();
	let tracing_config = movement_tracing::telemetry::Config::from_env()?;
	movement_tracing::telemetry::init_tracer_provider(
		env!("CARGO_PKG_NAME"),
		env!("CARGO_PKG_VERSION"),
		tracing_config,
	)?;

	// get the config file
	let dot_movement = dot_movement::DotMovement::try_from_env()?;
	let config_file = dot_movement.try_get_or_create_config_file().await?;

	let manager = Manager::new(config_file).await?;
	manager.try_run().await?;

	Ok(ExitCode::SUCCESS)
}
