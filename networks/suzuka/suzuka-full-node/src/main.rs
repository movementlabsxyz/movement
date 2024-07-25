use maptos_dof_execution::v1::Executor;
use suzuka_full_node::{manager::Manager, partial::SuzukaPartialNode};

use std::env;
use std::process::ExitCode;

const TIMING_ENV_VAR: &str = "SUZUKA_TIMING";
const TIMING_LOG_ENV_VAR: &str = "SUZUKA_TIMING_LOG";
const DEFAULT_TIMING_LOG_FILE: &str = "suzuka-full-node.timing.json";

fn main() -> Result<ExitCode, anyhow::Error> {
	let _guard = init_tracing_subscriber();

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

fn init_tracing_subscriber() -> Option<tracing_appender::non_blocking::WorkerGuard> {
	use std::{fs::File, path::PathBuf};
	use tracing_subscriber::filter::{EnvFilter, LevelFilter};
	use tracing_subscriber::fmt::format::FmtSpan;
	use tracing_subscriber::prelude::*;

	// TODO: compose console_subscriber as a layer
	let env_filter = EnvFilter::builder()
		.with_default_directive(LevelFilter::INFO.into())
		.from_env_lossy();
	let log_layer = tracing_subscriber::fmt::layer().with_filter(env_filter);

	let (timing_layer, timing_writer_guard) =
		if let Ok(timing_directives) = env::var(TIMING_ENV_VAR) {
			let env_filter = EnvFilter::try_new(timing_directives);
			let timing_log_path: PathBuf = env::var_os(TIMING_LOG_ENV_VAR)
				.unwrap_or_else(|| DEFAULT_TIMING_LOG_FILE.into())
				.into();
			let timing_log_file = File::create(&timing_log_path);
			match (env_filter, timing_log_file) {
				(Ok(env_filter), Ok(file)) => {
					let (writer, guard) = tracing_appender::non_blocking(file);
					let layer = tracing_subscriber::fmt::layer()
						.with_writer(writer)
						.json()
						.with_span_events(FmtSpan::CLOSE)
						.with_filter(env_filter);
					(Some(layer), Some(guard))
				}
				(Err(e), _) => {
					eprintln!("invalid value of {TIMING_ENV_VAR}: {e}");
					(None, None)
				}
				(_, Err(e)) => {
					eprintln!("can't create `{}`: {}", timing_log_path.display(), e);
					(None, None)
				}
			}
		} else {
			(None, None)
		};

	tracing_subscriber::registry().with(log_layer).with(timing_layer).init();

	timing_writer_guard
}
