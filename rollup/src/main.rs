use aptos_config::config::{NodeConfig, WaypointConfig};
use aptos_event_notifications::EventSubscriptionService;
use aptos_infallible::RwLock;
use aptos_storage_interface::{DbReader, DbReaderWriter, DbWriter};
use aptos_temppath::TempPath;
use aptos_types::on_chain_config::{
	ApprovedExecutionHashes, ConfigID, OnChainConfig, OnChainConsensusConfig, ValidatorSet, Version,
};
use aptos_types::{chain_id::ChainId, waypoint::Waypoint};
use std::{fs, sync::Arc};

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
	let temp_path = TempPath::new();

	let mut node_config = NodeConfig::get_default_validator_config();
	node_config.set_data_dir(temp_path.path().to_path_buf());
	node_config.base.waypoint = WaypointConfig::FromConfig(Waypoint::default());

	// Disable mutual authentication for the config
	let validator_network = node_config.validator_network.as_mut().unwrap();
	validator_network.mutual_authentication = false;

	// Create an event subscription service
	let mut event_subscription_service =
		EventSubscriptionService::new(Arc::new(RwLock::new(DbReaderWriter::new(MockDatabase {}))));

	// Set up the networks and gather the application network handles. This should panic.
	let peers_and_metadata = aptos_node::network::create_peers_and_metadata(&node_config);
	let _ = network::setup_networks_and_get_interfaces(
		&node_config,
		ChainId::test(),
		peers_and_metadata,
		&mut event_subscription_service,
	);
}
