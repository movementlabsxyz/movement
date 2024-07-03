use aptos_api::Context;
use poem::{handler, web::Data, IntoResponse, Response};
use std::sync::Arc;

#[handler]
pub async fn health() -> Response {
	"OK".into_response()
}

#[handler]
pub async fn richard() -> Response {
	"Well Done".into_response()
}

#[handler]
pub async fn get_current_commitment(
	context: Data<&Arc<Context>>,
) -> Result<Response, anyhow::Error> {
	let latest_ledger_info = context.db.get_latest_ledger_info()?;
	let version = latest_ledger_info.ledger_info().version();
	let state_proof = context.db.get_state_proof(version)?;
	let commitment = movement_types::Commitment::digest_state_proof(&state_proof);
	Ok(hex::encode(&commitment.0).into_response())
}

#[handler]
pub async fn get_finalized_block_info(
	context: Data<&Arc<Context>>,
) -> Result<Response, anyhow::Error> {
	let latest_ledger_info = context.db.get_latest_ledger_info()?;
	let latest_block_info = latest_ledger_info.ledger_info().commit_info();
	Ok(serde_json::to_string(&latest_block_info)?.into_response())
}

#[cfg(test)]
mod tests {
	use super::*;
	use poem::get;
	use poem::middleware::Tracing;
	use poem::test::TestClient;
	use poem::EndpointExt;
	use poem::Route;

	#[tokio::test]
	async fn test_health_endpoint() {
		// Create a test client
		let client = TestClient::new(create_routes());

		// Test the /health endpoint
		let response = client.get("/health").send().await;
		assert!(response.0.status().is_success());
	}

	pub fn create_routes() -> impl EndpointExt {
		Route::new().at("/health", get(health)).with(Tracing)
	}
}
