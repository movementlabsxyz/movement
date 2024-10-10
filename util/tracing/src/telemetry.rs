//! OpenTelemetry support for Movement services.
//!
//! Telemetry is currently being exported to components as an API distinct
//! from the tracing framework, due to [issues][tracing-opentelemetry#159]
//! with integrating OpenTelemetry as a tracing subscriber.
//!
//! [tracing-opentelemetry#159]: https://github.com/tokio-rs/tracing-opentelemetry/issues/159

use crate::Config;

use opentelemetry::{trace::TracerProvider as _, KeyValue};
use opentelemetry_otlp::WithExportConfig as _;
use opentelemetry_sdk::trace::{Config as TraceConfig, TracerProvider};
use opentelemetry_sdk::{runtime, Resource};
use opentelemetry_semantic_conventions::resource::{SERVICE_NAME, SERVICE_VERSION};
use tracing::{error, Level, Subscriber};
use tracing_opentelemetry::OpenTelemetryLayer;
use tracing_subscriber::filter;
use tracing_subscriber::prelude::*;
use tracing_subscriber::registry::LookupSpan;

/// The scope guard object for the background tasks of the tracing subsystem.
///
/// This object needs to be kept alive for the duration of the program.
#[must_use = "should be dropped at the end of the program scope"]
#[derive(Debug)]
pub struct ScopeGuard(Option<TracerProvider>);

impl Drop for ScopeGuard {
	fn drop(&mut self) {
		if let Some(tracer_provider) = &self.0 {
			// Make sure all batched traces are exported.
			if let Err(e) = tracer_provider.shutdown() {
				error!("OpenTelemetry tracer provider shutdown failed: {e}");
			}
		}
	}
}

/// Adds an optional OpenTelemetry tracing layer to the provided subscriber.
///
/// This function should be called at the start of the program before any
/// threads are able to use OpenTelemetry tracers. The function will panic
/// if not called within a Tokio runtime.
pub(crate) fn init_tracing_layer<S>(
	subscriber: S,
	service_name: &'static str,
	service_version: &'static str,
	config: &Config,
) -> Result<(ScopeGuard, impl Subscriber), anyhow::Error>
where
	S: Subscriber,
	for<'span> S: LookupSpan<'span>,
{
	let (tracer_provider, layer) = if let Some(endpoint) = &config.otlp_grpc_url {
		let exporter = opentelemetry_otlp::new_exporter().tonic().with_endpoint(endpoint);
		let provider = opentelemetry_otlp::new_pipeline()
			.tracing()
			.with_exporter(exporter)
			.with_trace_config(TraceConfig::default().with_resource(Resource::new([
				KeyValue::new(SERVICE_NAME, service_name),
				KeyValue::new(SERVICE_VERSION, service_version),
			])))
			.install_batch(runtime::Tokio)?;
		let layer = OpenTelemetryLayer::new(provider.tracer("movement"))
			.with_filter(filter::Targets::new().with_target("movement_telemetry", Level::INFO));
		(Some(provider), Some(layer))
	} else {
		(None, None)
	};
	Ok((ScopeGuard(tracer_provider), subscriber.with(layer)))
}
