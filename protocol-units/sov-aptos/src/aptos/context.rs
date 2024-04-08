use anyhow;
use aptos_storage_interface::state_view::{DbStateView, DbStateViewAtVersion};
use aptos_storage_interface::DbReader;
use aptos_types::chain_id::ChainId;
use aptos_types::transaction::Version;
use std::sync::Arc;

/// Context holds Aptos scope context this is a stripped back and adapted version
/// of the `aptos_api::context::Context` struct to make it compatible with Sovereign Labs
/// whilst still being able to interact with the Aptos VM.
#[derive(Clone)]
pub struct Context {
	chain_id: ChainId,
	pub db: Arc<dyn DbReader>,
	// mp_sender: MempoolClientSender,
	// pub node_config: Arc<NodeConfig>,
	// gas_schedule_cache: Arc<RwLock<GasScheduleCache>>,
	// gas_estimation_cache: Arc<RwLock<GasEstimationCache>>,
	// gas_limit_cache: Arc<RwLock<GasLimitCache>>,
	// view_function_stats: Arc<FunctionStats>,
	// simulate_txn_stats: Arc<FunctionStats>,
	// pub table_info_reader: Option<Arc<dyn TableInfoReader>>,
}

impl Context {
	pub fn new(chain_id: ChainId, db: Arc<dyn DbReader>) -> Self {
		Self { chain_id, db }
	}

	pub fn state_view_at_version(&self, version: Version) -> Result<DbStateView, anyhow::Error> {
		Ok(self.db.state_view_at_version(Some(version))?)
	}
}
