use aptos_telemetry;
use aptos_config::config::NodeConfig;
use aptos_types::chain_id::ChainId;
use std::collections::BTreeMap;
use once_cell::sync::Lazy;
use aptos_logger::{info, warn};
use hex;
use aptos_config::config::RoleType;

// Load config
static SUZUKA_CONFIG: Lazy<movement_config::Config> = Lazy::new(|| {
    let dot_movement = dot_movement::DotMovement::try_from_env().unwrap();
    let config = dot_movement.try_get_config_from_json::<movement_config::Config>().unwrap();
    config
});

fn init_telemetry_env() {
    // Making sure to remove vars that might disable metrics
    std::env::remove_var("APTOS_DISABLE_TELEMETRY");
    std::env::remove_var("APTOS_DISABLE_PROMETHEUS_NODE_METRICS");
    
    // Force enabling metrics
    std::env::set_var("APTOS_FORCE_ENABLE_TELEMETRY", "1");
    std::env::set_var("PROMETHEUS_METRICS_ENABLED", "1");
    
    // Get node configuration from SUZUKA_CONFIG
    let node_connection_address = SUZUKA_CONFIG
        .execution_config
        .maptos_config
        .client
        .maptos_rest_connection_hostname
        .clone();
    let node_connection_port = SUZUKA_CONFIG
        .execution_config
        .maptos_config
        .client
        .maptos_rest_connection_port
        .clone();

    // Set up node URL for telemetry
    let node_url = format!("http://{}:{}", node_connection_address, node_connection_port);
    std::env::set_var("APTOS_NODE_URL", node_url);
    
    // Get chain ID from config
    let chain_id = SUZUKA_CONFIG.execution_config.maptos_config.chain.maptos_chain_id.id();
    std::env::set_var("APTOS_CHAIN_ID", chain_id.to_string());

    // Generate a node ID based on config values
    let node_id_input = format!("{}:{}:{}", node_connection_address, node_connection_port, chain_id);
    let node_id = hex::encode(ring::digest::digest(
        &ring::digest::SHA256,
        node_id_input.as_bytes(),
    ).as_ref());
    
    // Set the node ID for telemetry
    std::env::set_var("APTOS_TELEMETRY_NODE_ID_KEY", node_id.clone());
    std::env::set_var("APTOS_DISABLE_TELEMETRY_PUSH_METRICS", "0");
    info!("Using deterministic node ID for telemetry authentication: {}", node_id);
    
    std::env::set_var("APTOS_METRICS_PORT", "9464");
}

// Create a default NodeConfig and ChainId for telemetry
static DEFAULT_NODE_CONFIG: Lazy<NodeConfig> = Lazy::new(|| {
    init_telemetry_env();
    let mut config = NodeConfig::default();
    
    // Configure the node using SUZUKA_CONFIG values
    config.base.role = RoleType::FullNode;
    config.base.data_dir = "/tmp/aptos".into();
    
    config
});

static DEFAULT_CHAIN_ID: Lazy<ChainId> = Lazy::new(|| {
    // Get chain ID from movement config
    let chain_id_value = SUZUKA_CONFIG.execution_config.maptos_config.chain.maptos_chain_id.id();
    ChainId::new(chain_id_value)
});

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

/// Returns the metrics endpoint URL that Prometheus should scrape
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