use aptos_db::AptosDB;
use aptos_storage_interface::state_view::DbStateViewAtVersion;
use aptos_storage_interface::DbReader;
use aptos_types::on_chain_config::{OnChainConfig, OnChainExecutionConfig};
use aptos_types::state_store::StateView;
use std::env;
use std::path::Path;
use std::sync::Arc;

fn main() {
	// Get the current directory and construct absolute path
	let current_dir = env::current_dir().expect("Failed to get current directory");
	let db_path = current_dir.join(".movement/maptos/27/.maptos");

	// Check if directory exists
	if !db_path.exists() {
		panic!("Database directory does not exist: {}", db_path.display());
	}

	// Try to create parent directories if they don't exist
	if let Some(parent) = db_path.parent() {
		std::fs::create_dir_all(parent).expect("Failed to create parent directories");
	}

	// Open the database
	let db: Arc<dyn DbReader> = Arc::new(AptosDB::new_for_test(db_path.to_str().unwrap()));

	// Get the latest ledger version
	let latest_version = db.get_latest_ledger_info_version().expect("No ledger info found");

	// Get a state view at the latest version
	let state_view = db
		.state_view_at_version(Some(latest_version))
		.expect("Failed to get state view");

	// Try to fetch the OnChainExecutionConfig
	let config = OnChainExecutionConfig::fetch_config(&state_view);

	match config {
		Some(cfg) => {
			println!("OnChainExecutionConfig found: {:#?}", cfg);
		}
		None => {
			println!("OnChainExecutionConfig NOT found!");
		}
	}
}
