use aptos_telemetry;
use aptos_config::config::{NodeConfig, RoleType};
use aptos_types::chain_id::ChainId;
use std::collections::BTreeMap;
use once_cell::sync::Lazy;
use aptos_logger::{info, warn};
use prometheus::Encoder;  // Add this import
use tokio::runtime::Runtime;  // Remove Handle
use std::sync::Arc;
use warp::Filter;
use std::net::SocketAddr;

// Create a default NodeConfig for telemetry
static DEFAULT_NODE_CONFIG: Lazy<NodeConfig> = Lazy::new(|| {
    let mut config = NodeConfig::default();
    config.base.role = RoleType::FullNode;
    config.base.data_dir = "/tmp/aptos".into();
    config
});

static DEFAULT_CHAIN_ID: Lazy<ChainId> = Lazy::new(|| ChainId::new(1));
static DEFAULT_BUILD_INFO: Lazy<BTreeMap<String, String>> = Lazy::new(BTreeMap::new);

// Global runtime handle
static RUNTIME: Lazy<Arc<Runtime>> = Lazy::new(|| {
    Arc::new(Runtime::new().expect("Failed to create runtime"))
});

/// Initialize and start the telemetry service
pub fn start_telemetry_service() {
    // Configure environment variables for telemetry
    std::env::set_var("APTOS_FORCE_ENABLE_TELEMETRY", "1");
    std::env::set_var("PROMETHEUS_METRICS_ENABLED", "1");
    std::env::set_var("APTOS_METRICS_PORT", "9464");
    std::env::set_var("APTOS_METRICS_HOST", "0.0.0.0");
    std::env::set_var("APTOS_DISABLE_TELEMETRY_PUSH_METRICS", "1");
    
    let runtime = RUNTIME.clone();
    
    // Start the metrics server first
    let metrics_host = std::env::var("APTOS_METRICS_HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let metrics_port = std::env::var("APTOS_METRICS_PORT").unwrap_or_else(|_| "9464".to_string());
    let addr: SocketAddr = format!("{}:{}", metrics_host, metrics_port).parse().unwrap();
    
    // Create metrics handler
    let metrics_handler = || {
        let encoder = prometheus::TextEncoder::new();
        let mut buffer = vec![];
        encoder.encode(&prometheus::gather(), &mut buffer).unwrap();
        String::from_utf8(buffer).unwrap()
    };

    let metrics_route = warp::path!("metrics")
        .map(move || {
            let metrics = metrics_handler();
            warp::reply::with_header(
                metrics,
                "content-type",
                "text/plain; version=0.0.4",
            )
        });

    info!("Starting metrics server at http://{}", addr);
    
    // Spawn the metrics server
    runtime.spawn(async move {
        warp::serve(metrics_route)
            .run(addr)
            .await;
    });

    // Now start the telemetry service
    runtime.spawn(async move {
        let result = aptos_telemetry::service::start_telemetry_service(
            DEFAULT_NODE_CONFIG.clone(),
            *DEFAULT_CHAIN_ID,
            DEFAULT_BUILD_INFO.clone(),
            None,
            None,
        );

        match &result {
            Some(_) => info!("Telemetry service started successfully"),
            None => {
                warn!("Failed to start telemetry service. This might be expected if telemetry is disabled.");
                warn!("Metrics should still be available at the endpoint.");
            }
        }
    });
}

/// Ensures telemetry is initialized
pub fn ensure_telemetry_initialized() {
    start_telemetry_service();
}

/// Returns the metrics endpoint URL that Prometheus should scrape
pub fn get_metrics_endpoint() -> String {
    ensure_telemetry_initialized();
    "http://localhost:9464/metrics".to_string()
}

/// Shutdown the telemetry service
pub fn shutdown_telemetry() {
    info!("Shutting down telemetry service...");
    // The runtime will be dropped when the Arc count reaches 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_endpoint() {
        assert_eq!(get_metrics_endpoint(), "http://localhost:9464/metrics");
    }
}