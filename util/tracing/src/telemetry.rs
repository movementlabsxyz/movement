//! OpenTelemetry support for Movement services.
//!
//! Telemetry is currently being exported to components as an API distinct
//! from the tracing framework, due to [issues][tracing-opentelemetry#159]
//! with integrating OpenTelemetry as a tracing subscriber.
//!
//! [tracing-opentelemetry#159]: https://github.com/tokio-rs/tracing-opentelemetry/issues/159

use anyhow::anyhow;
use opentelemetry::global::{self, BoxedTracer};
use opentelemetry::trace::noop::NoopTracerProvider;
use opentelemetry::KeyValue;
use opentelemetry_otlp::WithExportConfig as _;
use opentelemetry_sdk::{runtime, trace::Config as TraceConfig, Resource};
use opentelemetry_semantic_conventions::resource::{SERVICE_NAME, SERVICE_VERSION};

use std::env;

const OTLP_TRACING_ENV: &str = "MOVEMENT_OTLP";

/// Options for telemetry configuration.
#[derive(Debug, Default)]
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

/// Global initialization of the OpenTelemetry tracer provider.
///
/// This function should be called at the start of the program before any
/// threads are able to use OpenTelemetry tracers. The function will panic
/// if not called within a Tokio runtime.
pub fn init_tracer_provider(
	service_name: &'static str,
	service_version: &'static str,
	config: Config,
) -> Result<(), anyhow::Error> {
	if let Some(endpoint) = config.otlp_grpc_url {
		dbg!(&endpoint);
		let exporter = opentelemetry_otlp::new_exporter().tonic().with_endpoint(endpoint);
		let provider = opentelemetry_otlp::new_pipeline()
			.tracing()
			.with_exporter(exporter)
			.with_trace_config(TraceConfig::default().with_resource(Resource::new([
				KeyValue::new(SERVICE_NAME, service_name),
				KeyValue::new(SERVICE_VERSION, service_version),
			])))
			.install_batch(runtime::Tokio)?;
		dbg!(&provider);
		global::set_tracer_provider(provider);
	} else {
		global::set_tracer_provider(NoopTracerProvider::new());
	}
	Ok(())
}

/// Get the tracer configured for the process.
pub fn tracer() -> BoxedTracer {
	global::tracer("movement")
}
