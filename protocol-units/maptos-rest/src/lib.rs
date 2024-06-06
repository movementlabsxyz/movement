use anyhow::Error;
use aptos_api::Context;
use poem::listener::TcpListener;
use poem::{
	get, handler,
	middleware::{AddData, Tracing},
	web::{Data, Path},
	EndpointExt, IntoResponse, Response, Route, Server,
};
use std::env;
use std::sync::Arc;
use tracing::info;

pub struct MaptosRest {
	/// The URL to bind the REST service to.
	pub url: String,
	pub context: Arc<Context>,
	// More fields to be added here, log verboisty, etc.
}

impl MaptosRest {
	pub const MAPTOS_REST_ENV_VAR: &'static str = "MAPTOS_REST_URL";

	pub fn try_from_env(context: Arc<Context>) -> Result<Self, Error> {
		let url =
			env::var(Self::MAPTOS_REST_ENV_VAR).unwrap_or_else(|_| "0.0.0.0:30832".to_string());
		Ok(Self { url, context })
	}

	pub async fn run_service(&self) -> Result<(), Error> {
		info!("Starting maptos rest service at {}", self.url);
		let maptos_rest = self.create_routes();
		Server::new(TcpListener::bind(&self.url)).run(maptos_rest).await?;
		Ok(())
	}

	pub fn create_routes(&self) -> impl EndpointExt {
		Route::new()
			.at("/health", get(health))
			.at("/movement/v1/state-root-hash/:version/:ledger_info_version", get(state_root_hash))
			.data(self.context.clone())
			.with(Tracing)
	}
}

#[handler]
async fn state_root_hash(
	Path(version): Path<u64>,
	Path(ledger_info_version): Path<u64>,
	context: Data<&Arc<Context>>,
) -> Result<Response, anyhow::Error> {
	let txn_with_proof =
		context.db.get_transaction_by_version(version, ledger_info_version, false)?;
	let state_root_hash = txn_with_proof
		.proof
		.transaction_info
		.state_checkpoint_hash()
		.ok_or_else(|| anyhow::anyhow!("No state root hash found"))?;
	Ok(state_root_hash.to_string().into_response())
}

#[handler]
async fn health() -> Response {
	"OK".into_response()
}
