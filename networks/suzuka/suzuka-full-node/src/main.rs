use suzuka_full_node::run::manager::Manager;

use std::env;
use std::process::ExitCode;

const TIMING_LOG_ENV: &str = "SUZUKA_TIMING_LOG";

#[tokio::main]
async fn main() -> Result<ExitCode, anyhow::Error> {
	let tracing_config =
		movement_tracing::Config { timing_log_path: env::var_os(TIMING_LOG_ENV).map(Into::into) };
	let _guard = movement_tracing::init_tracing_subscriber(tracing_config);

	// get the config file
	let dot_movement = dot_movement::DotMovement::try_from_env()?;
	let config_file = dot_movement.try_get_or_create_config_file().await?;

	let manager = Manager::new(config_file).await?;
	manager.try_run().await?;

	Ok(ExitCode::SUCCESS)
}
