use godfig::env_default;
use opentelemetry::{global, metrics::MeterProvider, KeyValue};
use opentelemetry_sdk::metrics::SdkMeterProvider;
use prometheus::{Encoder, Registry, TextEncoder};
use serde::{Deserialize, Serialize};
use std::{net::SocketAddr, sync::Arc};
use tokio::{net::TcpListener, sync::broadcast};
use tracing::error;
use tracing_subscriber::{
	filter::{EnvFilter, LevelFilter},
	fmt,
	prelude::*,
	Layer,
};
// The default metrics address hostname
env_default!(default_metrics_hostname, "MOVEMENT_METRICS_HOSTNAME", String, "0.0.0.0".to_string());

// The default metrics address port
env_default!(default_metrics_port, "MOVEMENT_METRICS_PORT", u16, 9464);

#[derive(Default, Serialize, Deserialize, Clone)]
pub struct Config {
	#[serde(default = "default_metrics_hostname")]
	pub metrics_hostname: String,
	#[serde(default = "default_metrics_port")]
	pub metrics_port: u16,
}

impl Config {
	pub fn default() -> Self {
		Self { metrics_hostname: default_metrics_hostname(), metrics_port: default_metrics_port() }
	}

	pub fn with_metrics_addr(mut self, addr: impl Into<String>) -> Self {
		let addr_str = addr.into();
		if let Ok(socket_addr) = addr_str.parse::<SocketAddr>() {
			self.metrics_hostname = socket_addr.ip().to_string();
			self.metrics_port = socket_addr.port();
		} else if let Some((host, port)) = addr_str.split_once(':') {
			if let Ok(port) = port.parse() {
				self.metrics_hostname = host.to_string();
				self.metrics_port = port;
			}
		}
		self
	}

	pub fn get_socket_addr(&self) -> String {
		format!("{}:{}", self.metrics_hostname, self.metrics_port)
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
			tokio::spawn(async move {
				let _ = server.await;
			});
		}
	}
}

pub fn init_tracing_subscriber(config: Config) -> WorkerGuard {
	let in_tokio_runtime = tokio::runtime::Handle::try_current().is_ok();

	if in_tokio_runtime {
		initialize_sync(config)
	} else {
		tokio::runtime::Runtime::new()
			.expect("Failed to create tokio runtime")
			.block_on(init_telemetry(config))
	}
}

fn initialize_sync(config: Config) -> WorkerGuard {
	let registry = Registry::new();
	let exporter = opentelemetry_prometheus::exporter()
		.with_registry(registry.clone())
		.build()
		.unwrap();

	let meter_provider = SdkMeterProvider::builder().with_reader(exporter).build();

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

	let subscriber = tracing_subscriber::registry().with(fmt::layer().with_filter(env_filter));

	tracing::subscriber::set_global_default(subscriber).expect("Failed to set tracing subscriber");

	let config_clone = config.clone();
	let registry = Arc::new(registry);
	let registry_clone = registry.clone();
	let metrics_server = tokio::spawn(async move {
		if let Err(e) = serve_metrics(&config_clone, registry_clone).await {
			tracing::error!("Metrics server error: {}", e);
		}
	});

	TelemetryGuard { _meter_provider: meter_provider, metrics_server: Some(metrics_server) }
}

pub async fn init_telemetry(config: Config) -> TelemetryGuard {
	let registry = Registry::new();
	let exporter = opentelemetry_prometheus::exporter()
		.with_registry(registry.clone())
		.build()
		.unwrap();

	let meter_provider = SdkMeterProvider::builder().with_reader(exporter).build();

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

	let subscriber = tracing_subscriber::registry().with(fmt::layer().with_filter(env_filter));

	tracing::subscriber::set_global_default(subscriber).expect("Failed to set tracing subscriber");

	let config_clone = config.clone();
	let registry = Arc::new(registry);
	let registry_clone = registry.clone();
	let metrics_server = tokio::spawn(async move {
		if let Err(e) = serve_metrics(&config_clone, registry_clone).await {
			tracing::error!("Metrics server error: {}", e);
		}
	});

	TelemetryGuard { _meter_provider: meter_provider, metrics_server: Some(metrics_server) }
}

pub async fn serve_metrics(
	config: &Config,
	registry: Arc<Registry>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
	let addr = match config.get_socket_addr().parse::<SocketAddr>() {
		Ok(addr) => addr,
		Err(e) => {
			error!("Failed to parse metrics address: {}", e);
			return Err(Box::new(e));
		}
	};

	println!("Attempting to bind metrics server to {}", addr);

	let listener = match TcpListener::bind(addr).await {
		Ok(l) => {
			println!("Metrics server successfully bound to {}", addr);
			l
		}
		Err(e) => {
			tracing::error!("CRITICAL ERROR: Failed to bind metrics server to {}: {}", addr, e);
			tracing::error!(
				"This may be because the port is already in use or you don't have permission."
			);
			return Err(Box::new(e));
		}
	};

	println!("Metrics server listening on {}", addr);

	let (shutdown_tx, mut shutdown_rx) = broadcast::channel(1);

	let shutdown_tx_clone = shutdown_tx.clone();
	tokio::spawn(async move {
		if let Err(e) = tokio::signal::ctrl_c().await {
			tracing::error!("Failed to listen for Ctrl+C: {}", e);
		}
		let _ = shutdown_tx_clone.send(());
	});

	loop {
		tokio::select! {
			accept_result = listener.accept() => {
				match accept_result {
					Ok((stream, client_addr)) => {
						let registry = Arc::clone(&registry);
						let shutdown_rx = shutdown_tx.subscribe();

						tokio::spawn(async move {
							if let Err(e) = handle_metrics_request(stream, client_addr, registry, shutdown_rx).await {
								tracing::error!("Error handling metrics request from {}: {}", client_addr, e);
							}
						});
					},
					Err(e) => {
						tracing::error!("Error accepting connection: {}", e);
					}
				}
			},

			_ = shutdown_rx.recv() => {
				println!("Metrics server received shutdown signal, exiting...");
				break;
			}
		}
	}

	println!("Metrics server has shut down gracefully");
	Ok(())
}

async fn handle_metrics_request(
	mut stream: tokio::net::TcpStream,
	_client_addr: std::net::SocketAddr,
	registry: Arc<Registry>,
	mut _shutdown_rx: broadcast::Receiver<()>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
	let metrics = registry.gather();
	let encoder = TextEncoder::new();
	let mut buffer = vec![];
	encoder.encode(&metrics, &mut buffer)?;

	let response = format!(
		"HTTP/1.1 200 OK\r\n\
        Content-Type: text/plain; charset=utf-8\r\n\
        Content-Length: {}\r\n\
        \r\n",
		buffer.len()
	);

	use tokio::io::AsyncWriteExt;

	stream.write_all(response.as_bytes()).await?;
	stream.write_all(&buffer).await?;
	stream.flush().await?;
	stream.shutdown().await?;

	Ok(())
}
