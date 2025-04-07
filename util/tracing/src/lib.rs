use opentelemetry::{
    global,
    metrics::MeterProvider,
    KeyValue,
};
use opentelemetry_sdk::metrics::SdkMeterProvider;
use prometheus::{Encoder, Registry, TextEncoder};
use std::{env, net::SocketAddr, sync::Arc};
use tokio::{
    net::TcpListener,
    sync::broadcast,
};
use tracing_subscriber::{
    fmt,
    prelude::*,
    filter::{EnvFilter, LevelFilter},
    Layer,
};

const METRICS_ADDR_ENV: &str = "MOVEMENT_METRICS_ADDR";
const DEFAULT_METRICS_ADDR: &str = "127.0.0.1:9464";

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
    
    let server_addr = metrics_addr.clone();
    let server_registry = registry.clone();
    let metrics_server = tokio::spawn(serve_metrics(server_addr, server_registry));

    TelemetryGuard {
        _meter_provider: meter_provider,
        metrics_server: Some(metrics_server),
    }
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
    println!("Attempting to bind metrics server to {}", addr);
    
    let listener = match TcpListener::bind(addr).await {
        Ok(l) => {
            println!("Metrics server successfully bound to {}", addr);
            l
        },
        Err(e) => {
            eprintln!("CRITICAL ERROR: Failed to bind metrics server to {}: {}", addr, e);
            eprintln!("This may be because the port is already in use or you don't have permission.");
            return;
        }
    };
    
    println!("Metrics server listening on {}", addr);

    let (shutdown_tx, mut shutdown_rx) = broadcast::channel(1);
    let registry = Arc::new(registry);
    
    let shutdown_tx_clone = shutdown_tx.clone();
    tokio::spawn(async move {
        if let Err(e) = tokio::signal::ctrl_c().await {
            eprintln!("Failed to listen for Ctrl+C: {}", e);
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
                                eprintln!("Error handling metrics request from {}: {}", client_addr, e);
                            }
                        });
                    },
                    Err(e) => {
                        eprintln!("Error accepting connection: {}", e);
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
}

async fn handle_metrics_request(
    mut stream: tokio::net::TcpStream,
    client_addr: std::net::SocketAddr,
    registry: Arc<Registry>,
    mut shutdown_rx: broadcast::Receiver<()>,
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

