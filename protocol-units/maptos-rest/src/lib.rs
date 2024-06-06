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
			.at("/movement/v1/state-root-hash/:blockheight", get(state_root_hash))
			.at("/movement/v1/contracts/state-root-hash/:blockheight" get(mcr_state_root_hash))
			.data(self.context.clone())
			.with(Tracing)
	}
}

#[handler]
async fn health() -> Response {
	"OK".into_response()
}

#[handler]
async fn state_root_hash(
	Path(blockheight): Path<u64>,
	context: Data<&Arc<Context>>,
) -> Result<Response, anyhow::Error> {
	let latest_ledger_info = context.db.get_latest_ledger_info()?;
	let (_, end_version, _) = context.db.get_block_info_by_height(blockheight)?;
	let txn_with_proof = context.db.get_transaction_by_version(
		end_version,
		latest_ledger_info.ledger_info().version(),
		false,
	)?;
	let state_root_hash = txn_with_proof
		.proof
		.transaction_info
		.state_checkpoint_hash()
		.ok_or_else(|| anyhow::anyhow!("No state root hash found"))?;
	Ok(state_root_hash.to_string().into_response())
}

//These feel more like integration tests, can be moved to a different
//file if we like.
// #[cfg(test)]
// mod tests {
// 	use super::*;
// 	use anyhow::Error;
// 	use aptos_api::Context;
// 	use poem::test::TestClient;
// 	use std::sync::Arc;
//
// 	#[tokio::test]
// 	async fn test_health_endpoint() -> Result<(), Error> {
// 		let node = SuzukaPartialNode::try_from_env().await?;
//
// 		let response = client.get("/health").send().await;
//
// 		assert_eq!(response.status().as_u16(), 200);
// 		assert_eq!(response.text().await.unwrap(), "OK");
//
// 		Ok(())
// 	}
//
// 	#[tokio::test]
// 	async fn test_state_root_hash_endpoint() -> Result<(), Error> {
// 		let context = Arc::new(Context {
//             // Initialize your context fields
//         });
// 		let service = MaptosRest::try_from_env(context.clone()).unwrap();
// 		let app = service.create_routes();
// 		let client = TestClient::new(app);
//
// 		let blockheight: u64 = 1; // Replace with an appropriate blockheight for testing
// 		let response =
// 			client.get(format!("/movement/v1/state-root-hash/{}", blockheight)).send().await;
//
// 		assert_eq!(response.status().as_u16(), 200);
// 		let state_root_hash = response.text().await.unwrap();
// 		println!("State root hash: {}", state_root_hash);
//
// 		Ok(())
// 	}
// }
