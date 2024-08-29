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
	tracing::info!("get_finalized_block_info before get latest_ledger_info");
	let latest_ledger_info = context.db.get_latest_ledger_info()?;
	tracing::info!("get_finalized_block_info get latest_ledger_info");
	let latest_block_info = latest_ledger_info.ledger_info().commit_info();
	tracing::info!("get_finalized_block_info latest_block_info {:?}", latest_block_info);
	Ok(serde_json::to_string(&latest_block_info)?.into_response())
}

// #[handler]
// pub async fn state_root_hash(
// 	Path(blockheight): Path<u64>,
// 	context: Data<&Arc<Context>>,
// ) -> Result<Response, anyhow::Error> {
// 	let latest_ledger_info = context.db.get_latest_ledger_info()?;
// 	let (_, end_version, _) = context.db.get_block_info_by_height(blockheight)?;
// 	tracing::info!("end_version: {}", end_version);
// 	let txn_with_proof = context.db.get_transaction_by_version(
// 		end_version,
// 		latest_ledger_info.ledger_info().version(),
// 		false,
// 	)?;
// 	tracing::info!("txn_with_proof: {:?}", txn_with_proof);
// 	let state_root_hash = txn_with_proof
// 		.proof
// 		.transaction_info
// 		.state_checkpoint_hash()
// 		.ok_or_else(|| anyhow::anyhow!("No state root hash found"))?;
// 	Ok(state_root_hash.to_string().into_response())
// }
