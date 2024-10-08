use anyhow::anyhow;
use opentelemetry::trace::TracerProvider as _;
use opentelemetry_otlp::WithExportConfig as _;
use opentelemetry_sdk::runtime;
use tracing_subscriber::filter::{self, EnvFilter, LevelFilter};
use tracing_subscriber::prelude::*;

use std::env;

const OTLP_TRACING_ENV: &str = "MOVEMENT_OTLP";

/// Options for the tracing subscriber.
#[derive(Default)]
pub struct Config {
	/// URL of the collector endpoint using the OTLP gRPC protocol.
	pub otlp_grpc_url: Option<String>,
}

impl Config {
	/// Get the tracing configuration from well-known environment variables.
	pub fn from_env() -> Result<Self, anyhow::Error> {
		let otlp_grpc_url = match env::var(OTLP_TRACING_ENV) {
			Ok(url) => Some(url),
			Err(env::VarError::NotPresent) => None,
			Err(env::VarError::NotUnicode(s)) => {
				return Err(anyhow!(
					"value of environment variable {OTLP_TRACING_ENV} is not valid UTF-8: {}",
					s.to_string_lossy()
				));
			}
		};
		Ok(Self { otlp_grpc_url })
	}
}

/// Sets up the tracing subscribers for a Movement process. This should be
/// called at the beginning of a process' `main` function.
pub fn init_tracing_subscriber(config: Config) -> Result<(), anyhow::Error> {
	// TODO: compose console_subscriber as a layer
	let env_filter = EnvFilter::builder()
		.with_default_directive(LevelFilter::INFO.into())
		.from_env_lossy();
	let log_layer = tracing_subscriber::fmt::layer().with_filter(env_filter);

	let telemetry_layer = if let Some(endpoint) = config.otlp_grpc_url {
		let exporter = opentelemetry_otlp::new_exporter().tonic().with_endpoint(endpoint);
		let tracer = opentelemetry_otlp::new_pipeline()
			.tracing()
			.with_exporter(exporter)
			.install_batch(runtime::Tokio)?
			.tracer("movement_tracing");
		let layer = tracing_opentelemetry::layer()
			.with_tracer(tracer)
			.with_filter(filter::filter_fn(|meta| meta.target() == "movement_telemetry"));
		Some(layer)
	} else {
		None
	};

	tracing_subscriber::registry().with(log_layer).with(telemetry_layer).init();

	Ok(())
}
