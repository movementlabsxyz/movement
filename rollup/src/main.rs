use aptos_config::config::{NodeConfig, WaypointConfig};
use aptos_event_notifications::EventSubscriptionService;
use aptos_infallible::RwLock;
use aptos_node::network;
use aptos_storage_interface::{DbReader, DbReaderWriter, DbWriter};
use aptos_temppath::TempPath;
use aptos_types::{
	chain_id::ChainId,
	on_chain_config::{
		ApprovedExecutionHashes, ConfigID, OnChainConfig, OnChainConsensusConfig, ValidatorSet,
		Version,
	},
	waypoint::Waypoint,
};
use log::info;
use std::sync::Arc;
/// State sync will panic if the value of any config in this registry is uninitialized
pub const ON_CHAIN_CONFIG_REGISTRY: &[ConfigID] = &[
	ApprovedExecutionHashes::CONFIG_ID,
	ValidatorSet::CONFIG_ID,
	Version::CONFIG_ID,
	OnChainConsensusConfig::CONFIG_ID,
	ChainId::CONFIG_ID,
];

/// A mock database implementing DbReader and DbWriter
pub struct MockDatabase;
impl DbReader for MockDatabase {}
impl DbWriter for MockDatabase {}

fn main() {
	env_logger::init();
	let mut node_config = NodeConfig::load_from_path("rollup/test_data/validator.yaml")
		.expect("Failed to load node config");
	info!("Node config: {:?}", node_config);
	node_config.base.waypoint = WaypointConfig::FromConfig(Waypoint::default());
	info!("way point set");
	// Create an event subscription service
	let mut event_subscription_service =
		EventSubscriptionService::new(Arc::new(RwLock::new(DbReaderWriter::new(MockDatabase {}))));
	info!("event subscription service created");

	// Set up the networks and gather the application network handles. This should panic.
	let peers_and_metadata = network::create_peers_and_metadata(&node_config);
	info!("peers and metadata created");
	let _ = network::setup_networks_and_get_interfaces(
		&node_config,
		ChainId::test(),
		peers_and_metadata,
		&mut event_subscription_service,
	);
	info!("networks setup and interfaces created");
}
