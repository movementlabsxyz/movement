// service/src/telemetry.rs

use opentelemetry::sdk::{export::metrics::aggregation, metrics::controllers, Resource};
use opentelemetry::KeyValue;
use opentelemetry_jaeger;
use opentelemetry_otlp::ExporterConfig;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::Registry;

pub fn init_telemetry() -> Result<(), Box<dyn std::error::Error>> {
        // Initialize Jaeger tracer for distributed tracing
        let tracer = opentelemetry_jaeger::new_pipeline()
                .with_service_name("relayer")
                .install_simple()?;
        
        // OpenTelemetry tracing layer and subscriber
        let telemetry = tracing_opentelemetry::layer().with_tracer(tracer);
        let subscriber = Registry::default().with(telemetry);

        // Set the global subscriber to handle tracing
        tracing::subscriber::set_global_default(subscriber)?;

        // Configure OTLP exporter to send metrics
        let resource = Resource::new(vec![KeyValue::new("service.name", "relayer")]);
        let controller = controllers::basic(aggregation::cumulative_temporality_selector())
                .with_resource(resource)
                .build();
        opentelemetry::global::set_meter_provider(controller.provider());

        Ok(())
}
