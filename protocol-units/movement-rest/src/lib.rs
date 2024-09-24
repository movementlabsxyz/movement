use aptos_api::Context;

use anyhow::Error;
use aptos_types::{
	account_address::AccountAddress,
	aggregate_signature::AggregateSignature,
	block_info::BlockInfo,
	epoch_change::EpochChangeProof,
	ledger_info::{LedgerInfo, LedgerInfoWithSignatures},
	proof::TransactionInfoWithProof,
	state_proof::StateProof,
	state_store::{state_key::StateKey, table::TableHandle},
};
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
			.unwrap_or_else(|_| "0.0.0.0:30832".to_string());
		Ok(Self { url, context: None })
	}

	pub fn set_context(&mut self, context: Arc<Context>) {
		self.context = Some(context);
	}

	pub fn run_service(&self) -> impl Future<Output = anyhow::Result<()>> + Send {
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
			.at(
				"/movement/v1/table-item-with-proof/:table_handle/:key/:blockheight",
				get(table_item_with_proof),
			)
			.at("/movement/v1/state-proof/:blockheight", get(state_proof))
			.data(self.context.as_ref().unwrap().clone())
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

/// Get a table item with the (non)inclusion proof.
///
/// Returns a tuple of value and (non)inclusion proof.
///
/// Also see [`https://aptos.dev/en/build/apis/fullnode-rest-api-reference?network=mainnet#tag/tables/POST/tables/{table_handle}/item`].
#[handler]
pub async fn table_item_with_proof(
	Path((table_handle, key, blockheight)): Path<(AccountAddress, String, u64)>,
	context: Data<&Arc<Context>>,
) -> Result<Response, anyhow::Error> {
	let (_, end_version, _) = context.db.get_block_info_by_height(blockheight)?;

	let key = hex::decode(&key)?;
	let key = StateKey::table_item(&TableHandle(table_handle), &key);

	let resp = context.db.get_state_value_with_proof_by_version(&key, end_version)?;

	Ok(serde_json::to_string(&resp)?.into_response())
}

/// Get the `StateProof` at a specific height. Note that this doesn't give you the latest `StateProof` against the latest ledger info.
/// This gives you the `StateProof` that is committed to L1 at a specific height.
#[handler]
pub async fn state_proof(
	Path(blockheight): Path<u64>,
	context: Data<&Arc<Context>>,
) -> Result<Response, anyhow::Error> {
	#[derive(serde::Serialize, serde::Deserialize)]
	struct StateProofResponse {
		tx_index: u64,
		state_proof: StateProof,
		tx_proof: TransactionInfoWithProof,
	}

	let (_, end_version, block_event) = context.db.get_block_info_by_height(blockheight)?;

	let mut epoch_state = context.db.get_latest_epoch_state()?;
	epoch_state.epoch = block_event.epoch();

	// We are reconstructing `StateProof` from scratch since Aptos lacks the api to fetch the `StateProof` at a specific height.
	let block_info = BlockInfo::new(
		block_event.epoch(),
		block_event.round(),
		block_event.hash()?,
		context.db.get_accumulator_root_hash(end_version)?,
		end_version,
		block_event.timestamp,
		Some(epoch_state),
	);

	// `consensus_data_hash` and `signatures` are always meant to be empty.
	let ledger_info = LedgerInfoWithSignatures::new(
		LedgerInfo::new(block_info, Default::default()),
		AggregateSignature::empty(),
	);

	// Epoch change is empty as well since `StateProof` is always calculated at the latest state which
	// means no epoch change exists.
	let state_proof = StateProof::new(ledger_info, EpochChangeProof::new(vec![], false));

	let tx = context.db.get_transaction_by_version(end_version, end_version, false)?;

	Ok(serde_json::to_string(&StateProofResponse {
		tx_index: tx.version,
		state_proof,
		tx_proof: tx.proof.clone(),
	})?
	.into_response())
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
