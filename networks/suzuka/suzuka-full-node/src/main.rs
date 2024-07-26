use maptos_dof_execution::v1::Executor;
use suzuka_full_node::{manager::Manager, partial::SuzukaPartialNode};

use std::env;
use std::process::ExitCode;

const TIMING_LOG_ENV: &str = "SUZUKA_TIMING_LOG";

fn main() -> Result<ExitCode, anyhow::Error> {
	let tracing_config =
		movement_tracing::Config { timing_log_path: env::var_os(TIMING_LOG_ENV).map(Into::into) };
	let _guard = movement_tracing::init_tracing_subscriber(tracing_config);

	let runtime = tokio::runtime::Builder::new_multi_thread().enable_all().build()?;

	if let Err(err) = runtime.block_on(run_suzuka()) {
		tracing::error!("Suzuka node main task exit with an error : {err}",);
	}

	// Terminate all running task.
	runtime.shutdown_background();
	Ok(ExitCode::SUCCESS)
}

async fn run_suzuka() -> Result<ExitCode, anyhow::Error> {
	// get the config file
	let dot_movement = dot_movement::DotMovement::try_from_env()?;
	let config_file = dot_movement.try_get_or_create_config_file().await?;

	let manager = Manager::<SuzukaPartialNode<Executor>>::new(config_file).await?;
	manager.try_run().await?;

	Ok(ExitCode::SUCCESS)
}
