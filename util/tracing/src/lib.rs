use tracing_appender::non_blocking::WorkerGuard as AppenderGuard;
use tracing_subscriber::filter::{self, EnvFilter, LevelFilter};
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::prelude::*;

use std::{env, fs::File, path::PathBuf};

const TIMING_ENV: &str = "MOVEMENT_TIMING";

/// The default path name for the timing log file.
/// If the path not specified in [`Config`] and the `MOVEMENT_TIMING`
/// environment variable is set, the log file with this name will be created.
pub const DEFAULT_TIMING_LOG_FILE: &str = "movement-timing.log";

/// A guard for background log appender(s) returned by `init_tracing_subscriber`.
pub struct WorkerGuard {
	_drop_me: Option<AppenderGuard>,
}

/// Options for the tracing subscriber.
#[derive(Default)]
pub struct Config {
	/// Custom name for the timing log file.
	pub timing_log_path: Option<PathBuf>,
}

/// Sets up the tracing subscribers for a Movement process. This should be
/// called at the beginning of a process' `main` function.
/// Returns a guard object that should be dropped at the end of the process'
/// `main`` function scope.
///
/// This function may output encounted errors to the standard error stream,
/// as this is the only facility
pub fn init_tracing_subscriber(config: Config) -> WorkerGuard {
	// TODO: compose console_subscriber as a layer
	let env_filter = EnvFilter::builder()
		.with_default_directive(LevelFilter::INFO.into())
		.from_env_lossy();
	let log_layer = tracing_subscriber::fmt::layer().with_filter(env_filter);

	let (timing_layer, timing_writer_guard) = match env::var(TIMING_ENV) {
		Err(env::VarError::NotPresent) => {
			// Disable timing
			(None, None)
		}
		Ok(timing_directives) => {
			let env_filter = EnvFilter::new(timing_directives);
			let timing_log_path = config
				.timing_log_path
				.as_deref()
				.unwrap_or_else(|| DEFAULT_TIMING_LOG_FILE.as_ref());
			match File::create(timing_log_path) {
				Ok(file) => {
					let (writer, guard) = tracing_appender::non_blocking(file);
					let layer = tracing_subscriber::fmt::layer()
						.with_writer(writer)
						.json()
						.with_span_events(FmtSpan::CLOSE)
						.with_filter(env_filter)
						.with_filter(filter::filter_fn(|meta| meta.target() == "movement_timing"));
					(Some(layer), Some(guard))
				}
				Err(e) => {
					eprintln!("can't create `{}`: {}", timing_log_path.display(), e);
					(None, None)
				}
			}
		}
		Err(e) => {
			eprintln!("invalid {TIMING_ENV}: {e}");
			(None, None)
		}
	};

	tracing_subscriber::registry().with(log_layer).with(timing_layer).init();

	WorkerGuard { _drop_me: timing_writer_guard }
}
