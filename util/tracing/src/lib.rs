use opentelemetry::{
	global,
	metrics::MeterProvider,
	KeyValue,
};
use opentelemetry_sdk::metrics::SdkMeterProvider;
use prometheus::{Encoder, Registry, TextEncoder};
use std::{env, net::SocketAddr};
use tokio::net::TcpListener;
use tracing_subscriber::{
	fmt,
	prelude::*,
	filter::{EnvFilter, LevelFilter},
	Layer,
};

const METRICS_ADDR_ENV: &str = "MOVEMENT_METRICS_ADDR";
const DEFAULT_METRICS_ADDR: &str = "0.0.0.0:9464";

#[derive(Default)]
pub struct Config {
	pub metrics_addr: Option<String>,
}

impl Config {
	pub fn new() -> Self {
		Self::default()
	}

	pub fn with_metrics_addr(addr: impl Into<String>) -> Self {
		Self {
			metrics_addr: Some(addr.into()),
		}
	}
}

pub struct TelemetryGuard {
	_meter_provider: SdkMeterProvider,
	metrics_server: Option<tokio::task::JoinHandle<()>>,
}

pub type WorkerGuard = TelemetryGuard;

impl Drop for TelemetryGuard {
	fn drop(&mut self) {
		if let Some(server) = self.metrics_server.take() {
			server.abort();
		}
	}
}

pub fn init_tracing_subscriber(config: Config) -> WorkerGuard {
	tokio::runtime::Runtime::new()
		.unwrap()
		.block_on(init_telemetry(config))
}

pub async fn init_telemetry(config: Config) -> TelemetryGuard {
	let registry = Registry::new();
	let exporter = opentelemetry_prometheus::exporter()
		.with_registry(registry.clone())
		.build()
		.unwrap();

	let meter_provider = SdkMeterProvider::builder()
		.with_reader(exporter)
		.build();

	let meter = meter_provider.meter("movement");

	let uptime_counter = meter
		.u64_counter("movement.uptime.seconds")
		.with_description("Service uptime in seconds")
		.build();

	let requests_histogram = meter
		.u64_histogram("movement.requests.duration")
		.with_description("Request duration in milliseconds")
		.build();

	uptime_counter.add(0, &[KeyValue::new("service", "movement")]);
	requests_histogram.record(0, &[KeyValue::new("service", "movement")]);

	global::set_meter_provider(meter_provider.clone());

	let env_filter = EnvFilter::builder()
		.with_default_directive(LevelFilter::INFO.into())
		.from_env_lossy();

	let subscriber = tracing_subscriber::registry()
		.with(fmt::layer().with_filter(env_filter));

	tracing::subscriber::set_global_default(subscriber)
		.expect("Failed to set tracing subscriber");

	let metrics_addr = config.metrics_addr
		.or_else(|| env::var(METRICS_ADDR_ENV).ok())
		.unwrap_or_else(|| DEFAULT_METRICS_ADDR.to_string());

	let metrics_server = tokio::spawn(serve_metrics(metrics_addr, registry));

	TelemetryGuard {
		_meter_provider: meter_provider,
		metrics_server: Some(metrics_server),
	}
}

async fn serve_metrics(addr: String, registry: Registry) {
	let addr: SocketAddr = addr.parse().expect("Invalid metrics address");
	let listener = TcpListener::bind(addr).await.expect("Failed to bind metrics server");
	println!("Metrics server listening on {}", addr);

	loop {
		if let Ok((mut stream, _)) = listener.accept().await {
			let registry = registry.clone();
			let encoder = TextEncoder::new();
			tokio::spawn(async move {
				let metrics = registry.gather();
				let mut buffer = vec![];
				encoder.encode(&metrics, &mut buffer).unwrap();
				tokio::io::copy(&mut &buffer[..], &mut stream).await.ok();
			});
		}
	}
}
