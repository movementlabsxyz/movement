use aptos_api::{Context, runtime::{Apis, get_apis}};
use aptos_config::config::NodeConfig;
use aptos_mempool::MempoolClientSender;
use aptos_storage_interface::{finality_view::FinalityView as AptosFinalityView, DbReader};
use maptos_execution_util::config::aptos::Config as AptosConfig;

use std::sync::Arc;

#[derive(Clone)]
/// The API view into the finalized state of the chain.
pub struct FinalityView {
	inner: Arc<AptosFinalityView<Arc<dyn DbReader>>>,
	context: Arc<Context>,
}

impl FinalityView {
	/// Create a new `FinalityView` instance.
	pub fn new(
		db_reader: Arc<dyn DbReader>,
		mempool_client_sender: MempoolClientSender,
		node_config: NodeConfig,
		aptos_config: &AptosConfig,
	) -> Self {
		let inner = Arc::new(AptosFinalityView::new(db_reader));
		let context = Arc::new(Context::new(
			aptos_config.chain_id.clone(),
			inner.clone(),
			mempool_client_sender,
			node_config,
			None,
		));
		Self { inner, context }
	}

	pub fn try_from_config(
		db_reader: Arc<dyn DbReader>,
		mempool_client_sender: MempoolClientSender,
		aptos_config: &AptosConfig,
	) -> Result<Self, anyhow::Error> {
		let node_config = NodeConfig::default();
		Ok(Self::new(db_reader, mempool_client_sender, node_config, aptos_config))
	}

    /// Update the finalized view with the latest block height.
	///
	/// The block must be found on the committed chain.
	pub fn set_finalized_block_height(
		&self,
		height: u64,
	) -> Result<(), anyhow::Error> {
		self.inner.set_finalized_block_height(height)?;
		Ok(())
	}

	pub fn get_apis(&self) -> Apis {
		get_apis(self.context.clone())
	}
}

#[cfg(test)]
mod tests {
	use super::*;
}
