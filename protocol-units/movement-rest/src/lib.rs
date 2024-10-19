use aptos_api::Context;

use anyhow::Error;
use futures::prelude::*;
use poem::listener::TcpListener;
use poem::{
	get, handler,
	middleware::Tracing,
	web::{Data, Path},
	EndpointExt, IntoResponse, Response, Route, Server,
};
use tracing::info;

use std::env;
use std::future::Future;
use std::sync::Arc;

#[derive(Debug)]
pub struct MovementRest {
	/// The URL to bind the REST service to.
	pub url: String,
	pub context: Option<Arc<Context>>,
	// More fields to be added here, log verboisty, etc.
}

impl MovementRest {
	pub const MOVEMENT_REST_ENV_VAR: &'static str = "MOVEMENT_REST_URL";

	pub fn try_from_env() -> Result<Self, Error> {
		let url = env::var(Self::MOVEMENT_REST_ENV_VAR)
			.unwrap_or_else(|_| "http://0.0.0.0:30832".to_string());
		Ok(Self { url, context: None })
	}

	pub fn set_context(&mut self, context: Arc<Context>) {
		self.context = Some(context);
	}

	pub fn run_service(&self) -> impl Future<Output = Result<(), Error>> + Send {
		info!("Starting movement rest service at {}", self.url);
		let movement_rest = self.create_routes();
		Server::new(TcpListener::bind(self.url.clone()))
			.run(movement_rest)
			.map_err(Into::into)
	}

	pub fn create_routes(&self) -> impl EndpointExt {
		Route::new()
			.at("/health", get(health))
			.at("/movement/v1/state-root-hash/:blockheight", get(state_root_hash))
			.at("movement/v1/richard", get(richard))
			.data(self.context.clone())
			.with(Tracing)
	}
}

#[handler]
pub async fn health() -> Response {
	"OK".into_response()
}

#[handler]
pub async fn richard() -> Response {
	"Well Done".into_response()
}

#[handler]
pub async fn state_root_hash(
	Path(blockheight): Path<u64>,
	context: Data<&Arc<Context>>,
) -> Result<Response, anyhow::Error> {
	let latest_ledger_info = context.db.get_latest_ledger_info()?;
	let (_, end_version, _) = context.db.get_block_info_by_height(blockheight)?;
	tracing::info!("end_version: {}", end_version);
	let txn_with_proof = context.db.get_transaction_by_version(
		end_version,
		latest_ledger_info.ledger_info().version(),
		false,
	)?;
	tracing::info!("txn_with_proof: {:?}", txn_with_proof);
	let state_root_hash = txn_with_proof
		.proof
		.transaction_info
		.state_checkpoint_hash()
		.ok_or_else(|| anyhow::anyhow!("No state root hash found"))?;
	Ok(state_root_hash.to_string().into_response())
}

#[cfg(test)]
mod tests {
	use super::*;
	use poem::test::TestClient;

	#[tokio::test]
	async fn test_health_endpoint() {
		let rest_service = MovementRest::try_from_env().expect("Failed to create MovementRest");
		assert_eq!(rest_service.url, "http://0.0.0.0:30832");
		// Create a test client
		let client = TestClient::new(rest_service.create_routes());

		// Test the /health endpoint
		let response = client.get("/health").send().await;
		assert!(response.0.status().is_success());
	}
}
