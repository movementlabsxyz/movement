use aptos_api::{
	runtime::{get_api_service, get_apis, Apis},
	Context,
};
use aptos_config::config::NodeConfig;
use aptos_mempool::MempoolClientSender;
use aptos_storage_interface::{finality_view::FinalityView as AptosFinalityView, DbReader};
use maptos_execution_util::config::aptos::Config as AptosConfig;

use poem::{http::Method, listener::TcpListener, middleware::Cors, EndpointExt, Route, Server};
use tracing::info;

use std::sync::Arc;

#[derive(Clone)]
/// The API view into the finalized state of the chain.
pub struct FinalityView {
	inner: Arc<AptosFinalityView<Arc<dyn DbReader>>>,
	context: Arc<Context>,
	listen_url: String,
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
		let listen_url = aptos_config.fin_listen_url.clone();
		Self { inner, context, listen_url }
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
	pub fn set_finalized_block_height(&self, height: u64) -> Result<(), anyhow::Error> {
		self.inner.set_finalized_block_height(height)?;
		Ok(())
	}

	pub fn get_apis(&self) -> Apis {
		get_apis(self.context.clone())
	}

	pub async fn run_service(&self) -> Result<(), anyhow::Error> {
		info!("Starting maptos-fin-view services at: {:?}", self.listen_url);

		let api_service =
			get_api_service(self.context.clone()).server(format!("http://{:?}", self.listen_url));

		let ui = api_service.swagger_ui();

		let cors = Cors::new()
			.allow_methods(vec![Method::GET, Method::POST])
			.allow_credentials(true);
		let app = Route::new().nest("/v1", api_service).nest("/spec", ui).with(cors);

		Server::new(TcpListener::bind(self.listen_url.clone()))
			.run(app)
			.await
			.map_err(|e| anyhow::anyhow!("Server error: {:?}", e))?;

		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use super::*;
}
