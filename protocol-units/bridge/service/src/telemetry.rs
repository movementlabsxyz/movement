// service/src/telemetry.rs
use anyhow::Result;
use opentelemetry_sdk::{trace::Config, runtime, Resource};
use opentelemetry::trace::TracerProvider;
use opentelemetry::KeyValue;
use opentelemetry_otlp::WithExportConfig;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::Registry;

pub fn init_telemetry() -> Result<()> {
        // Define OpenTelemetry resource attributes, such as service name
        let resource = Resource::new(vec![KeyValue::new("service.name", "relayer")]);

        // Configure OTLP trace pipeline
        let tracer_provider = opentelemetry_otlp::new_pipeline()
                .tracing()
                .with_exporter(opentelemetry_otlp::new_exporter().tonic().with_endpoint("http://localhost:4317"))
                .with_trace_config(Config::default().with_resource(resource.clone()))
                .install_batch(runtime::Tokio)?;

        // Get a Tracer using tracer_builder
        let tracer = tracer_provider.tracer_builder("relayer_tracer")
                .with_version(env!("CARGO_PKG_VERSION"))
                .build();

        // Set up tracing layer with OpenTelemetry
        let telemetry = tracing_opentelemetry::layer().with_tracer(tracer);
        let subscriber = Registry::default().with(telemetry);
        tracing::subscriber::set_global_default(subscriber)?;

        Ok(())
}
