use crate::telemetry::{self, ScopeGuard};
use crate::Config;
use tracing_subscriber::filter::{EnvFilter, LevelFilter};
use tracing_subscriber::prelude::*;
use tracing_subscriber::{fmt, Registry};

/// Sets up the tracing subscribers for a Movement process. This should be
/// called at the beginning of a process' `main` function.
///
/// If successful, returns a guard object that should be dropped at the end
/// of the process' `main` function scope.
pub fn init_tracing_subscriber(
	service_name: &'static str,
	service_version: &'static str,
	config: &Config,
) -> Result<ScopeGuard, anyhow::Error> {
	// TODO: compose console_subscriber as a layer
	let env_filter = EnvFilter::builder()
		.with_default_directive(LevelFilter::INFO.into())
		.from_env_lossy();
	let fmt_layer = fmt::layer().with_filter(env_filter);
	let subscriber = Registry::default().with(fmt_layer);
	let (scope_guard, subscriber) =
		telemetry::init_tracing_layer(subscriber, service_name, service_version, config)?;
	subscriber.init();
	Ok(scope_guard)
}
