use tracing_subscriber::filter::{EnvFilter, LevelFilter};

/// Sets up the tracing subscribers for a Movement process. This should be
/// called at the beginning of a process' `main` function.
pub fn init_tracing_subscriber() {
	// TODO: compose console_subscriber as a layer
	let env_filter = EnvFilter::builder()
		.with_default_directive(LevelFilter::INFO.into())
		.from_env_lossy();
	tracing_subscriber::fmt().with_env_filter(env_filter).init();
}
