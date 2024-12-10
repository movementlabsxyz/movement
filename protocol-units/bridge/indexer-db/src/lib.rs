use crate::client::Client;
use bridge_config::Config;
use bridge_util::chains::bridge_contracts::BridgeContractMonitoring;
use bridge_util::types::BridgeTransferId;
use bridge_util::TransferActionType;
use tokio::select;
use tokio::sync::mpsc;
use tokio_stream::StreamExt;

pub mod client;
pub mod migrations;
pub mod models;
pub mod schema;

pub async fn run_indexer_client<
	SOURCE: Send + TryFrom<Vec<u8>> + std::clone::Clone + 'static + std::fmt::Debug,
	TARGET: Send + TryFrom<Vec<u8>> + std::clone::Clone + 'static + std::fmt::Debug,
>(
	config: Config,
	mut stream_source: impl BridgeContractMonitoring<Address = SOURCE>,
	mut stream_target: impl BridgeContractMonitoring<Address = TARGET>,
	relayer_actions: Option<mpsc::Sender<(BridgeTransferId, TransferActionType)>>,
) -> Result<(), anyhow::Error>
where
	Vec<u8>: From<SOURCE>,
	Vec<u8>: From<TARGET>,
{
	let mut indexer_db_client = match Client::from_bridge_config(&config) {
		Ok(mut client) => {
			client.run_migrations()?;
			client
		}
		Err(e) => {
			panic!("Failed to create indexer db client: {e:?}");
		}
	};

	loop {
		select! {
			// Wait on chain source events.
			Some(event_res) = stream_source.next() =>{
				if let Err(err) = event_res.map_err(|err| err.to_string()).and_then(|event| {
					indexer_db_client
						.insert_bridge_contract_event(event)
						.map_err(|err| err.to_string())
				}) {
					tracing::error!("Indexer: new event integration return an error:{err}")
				}
			}
			// Wait on chain target events.
			Some(event_res) = stream_target.next() =>{
				if let Err(err) = event_res.map_err(|err| err.to_string()).and_then(|event| {
					indexer_db_client
						.insert_bridge_contract_event(event)
						.map_err(|err| err.to_string())
				}) {
					tracing::error!("Indexer: new event integration return an error:{err}")
				}
			}
		}
	}
}
