use maptos_dof_execution::v1::Executor;
use suzuka_full_node::{manager::Manager, partial::SuzukaPartialNode};

use tracing_subscriber::{filter::LevelFilter, fmt::format::FmtSpan, EnvFilter};

use std::env;
use std::process::ExitCode;

const TIMING_ENV_VAR: &str = "SUZUKA_TIMING";

fn main() -> Result<ExitCode, anyhow::Error> {
	init_tracing_subscriber();

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

fn init_tracing_subscriber() {
	// TODO: compose console_subscriber as a layer
	let env_filter = EnvFilter::builder()
		.with_default_directive(LevelFilter::INFO.into())
		.from_env_lossy();
	let mut subscriber = tracing_subscriber::fmt().with_env_filter(env_filter);
	if env::var(TIMING_ENV_VAR).map_or(false, |v| v != "0") {
		subscriber = subscriber.with_span_events(FmtSpan::CLOSE);
	}
	subscriber.init()
}
