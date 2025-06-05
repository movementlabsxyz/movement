//! Telemetry crate for collecting metrics from aptos-core components.

use aptos_telemetry;
use aptos_config::config::NodeConfig;
use aptos_types::chain_id::ChainId;
use aptos_crypto::traits::ValidCryptoMaterial;
use std::collections::BTreeMap;
use once_cell::sync::Lazy;
use aptos_logger::{info, warn};
use hex;

fn init_telemetry_env() {
    // Making sure to remove vars that might disable metrics
    std::env::remove_var("APTOS_DISABLE_TELEMETRY");
    std::env::remove_var("APTOS_DISABLE_PROMETHEUS_NODE_METRICS");
    
    // Force enabling metrics
    std::env::set_var("APTOS_FORCE_ENABLE_TELEMETRY", "1");
    std::env::set_var("PROMETHEUS_METRICS_ENABLED", "1");
    
    // Enable metrics pushing if APTOS_TELEMETRY_NODE_ID_KEY is set - this is for debugging the priv key problem
    if std::env::var("APTOS_TELEMETRY_NODE_ID_KEY").is_ok() {
        std::env::set_var("APTOS_DISABLE_TELEMETRY_PUSH_METRICS", "0");
    } else {
        std::env::set_var("APTOS_DISABLE_TELEMETRY_PUSH_METRICS", "1");
        warn!("APTOS_TELEMETRY_NODE_ID_KEY not set, metrics pushing will be disabled");
    }
    
    std::env::set_var("APTOS_METRICS_PORT", "9464");
}

// Create a default NodeConfig and ChainId for telemetry
static DEFAULT_NODE_CONFIG: Lazy<NodeConfig> = Lazy::new(|| {
    init_telemetry_env();
    let mut config = NodeConfig::default();
    
    if let Some(identity_key) = config.get_identity_key() {
        let key_bytes = identity_key.to_bytes();
        std::env::set_var("APTOS_TELEMETRY_NODE_ID_KEY", hex::encode(&key_bytes));
        info!("Using node identity key for telemetry authentication");
    } else {
        warn!("No node identity key found, telemetry metrics pushing will be disabled");
        std::env::set_var("APTOS_DISABLE_TELEMETRY_PUSH_METRICS", "1");
    }
    
    config
});

static DEFAULT_CHAIN_ID: Lazy<ChainId> = Lazy::new(|| ChainId::new(1));
static DEFAULT_BUILD_INFO: Lazy<BTreeMap<String, String>> = Lazy::new(BTreeMap::new);

// This will automatically start the telemetry service when the crate is used
static TELEMETRY_RUNTIME: Lazy<Option<tokio::runtime::Runtime>> = Lazy::new(|| {
    info!("Initializing telemetry service...");
    

    init_telemetry_env();
    
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

    result
});

/// Ensures telemetry is initialized
pub fn ensure_telemetry_initialized() {
    init_telemetry_env();
    Lazy::force(&TELEMETRY_RUNTIME);
}

/// Returns the metrics endpoint URL that Prometheus should scrape (which is then added to grafana sources)
pub fn get_metrics_endpoint() -> String {
    info!("Forcing telemetry runtime initialization...");
    ensure_telemetry_initialized();
    "http://localhost:9464/metrics".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_endpoint() {
        assert_eq!(get_metrics_endpoint(), "http://localhost:9464/metrics");
    }
}