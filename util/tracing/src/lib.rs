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

// This function is safe to call from either synchronous or asynchronous contexts
pub fn init_tracing_subscriber(config: Config) -> WorkerGuard {
	// Detect if we're already in a Tokio runtime
	let in_tokio_runtime = tokio::runtime::Handle::try_current().is_ok();
	
	// Use a different strategy based on the context
	if in_tokio_runtime {
		// For celestia-da-light-node and other async contexts
		// Use a synchronous version of the initialization
		initialize_sync(config)
	} else {
		// For synchronous contexts
		tokio::runtime::Runtime::new()
			.expect("Failed to create tokio runtime")
			.block_on(init_telemetry(config))
	}
}

// Synchronous version of init_telemetry that doesn't use block_on
fn initialize_sync(config: Config) -> WorkerGuard {
	// Create registry and exporter synchronously
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

	println!("Metrics server starting on {}", metrics_addr);
	
	// Spawn the metrics server without using block_on
	let server_addr = metrics_addr.clone();
	let server_registry = registry.clone();
	let metrics_server = tokio::spawn(serve_metrics(server_addr, server_registry));

	TelemetryGuard {
		_meter_provider: meter_provider,
		metrics_server: Some(metrics_server),
	}
}

// The original async version for non-tokio contexts
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
	println!("Attempting to bind metrics server to {}", addr);
	let listener = match TcpListener::bind(addr).await {
		Ok(l) => {
			println!("Metrics server successfully bound to {}", addr);
			l
		},
		Err(e) => {
			eprintln!("CRITICAL ERROR: Failed to bind metrics server to {}: {}", addr, e);
			eprintln!("This may be because the port is already in use or you don't have permission.");
			// Continue with a panic to make the error more visible
			panic!("Failed to bind metrics server: {}", e);
		}
	};
	println!("Metrics server listening on {}", addr);

	loop {
		match listener.accept().await {
			Ok((mut stream, client_addr)) => {
				println!("Metrics server accepted connection from {}", client_addr);
				let registry = registry.clone();
				let encoder = TextEncoder::new();
				tokio::spawn(async move {
					let metrics = registry.gather();
					let mut buffer = vec![];
					encoder.encode(&metrics, &mut buffer).unwrap();
					match tokio::io::copy(&mut &buffer[..], &mut stream).await {
						Ok(_) => println!("Successfully served metrics to {}", client_addr),
						Err(e) => eprintln!("Error serving metrics to {}: {}", client_addr, e),
					}
				});
			},
			Err(e) => {
				eprintln!("Error accepting connection: {}", e);
			}
		}
	}
}
