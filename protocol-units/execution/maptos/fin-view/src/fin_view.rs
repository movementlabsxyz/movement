use aptos_api::{Context, runtime::{Apis, get_apis}};
use aptos_config::config::NodeConfig;
use aptos_mempool::MempoolClientSender;
use aptos_storage_interface::{finality_view::FinalityView as AptosFinalityView, DbReader};
use aptos_types::ledger_info::LedgerInfoWithSignatures;

use std::sync::Arc;

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
		aptos_config: maptos_execution_util::config::just_aptos::Config,
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

    /// Sets the ledger state as the latest finalized.
	///
	/// The ledger state must be found on the committed chain.
	pub fn set_finalized_ledger_info(
		&self,
		ledger_info_with_sigs: LedgerInfoWithSignatures,
	) -> Result<(), anyhow::Error> {
		self.inner.set_finalized_ledger_info(ledger_info_with_sigs)?;
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
