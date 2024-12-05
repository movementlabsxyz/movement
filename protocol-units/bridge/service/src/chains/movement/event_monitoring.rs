use super::{client_framework::FRAMEWORK_ADDRESS, utils::MovementAddress};
use crate::types::{Amount, BridgeAddress, BridgeTransferId};
use anyhow::Result;
use aptos_sdk::{
	rest_client::aptos_api_types::VersionedEvent, types::account_address::AccountAddress,
};
use bridge_config::common::movement::MovementConfig;
use bridge_util::chains::bridge_contracts::BridgeContractError;
use bridge_util::chains::bridge_contracts::BridgeContractEventType;
use bridge_util::chains::bridge_contracts::BridgeContractResult;
use bridge_util::chains::bridge_contracts::BridgeTransferCompletedDetails;
use bridge_util::chains::bridge_contracts::BridgeTransferInitiatedDetails;
use bridge_util::types::Nonce;
use bridge_util::BridgeContractEvent;
use bridge_util::BridgeContractMonitoring;

use futures::{
	channel::mpsc::{self as futurempsc},
	SinkExt, Stream, StreamExt,
};
use hex::FromHex;
use serde::{Deserialize, Deserializer, Serialize};
use std::{pin::Pin, task::Poll};
use tokio::fs::{self, File};
use tokio::io::{self, AsyncReadExt, AsyncWriteExt};
use tokio::sync::mpsc;
use tokio::sync::oneshot;

const PULL_STATE_FILE_NAME: &str = "pullstate.store";

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
struct MvtPullingState {
	initiated: u64,
	completed: u64,
}

impl MvtPullingState {
	async fn save_to_store_file(&self) -> io::Result<()> {
		let path = MvtPullingState::get_store_file_path();
		if let Some(parent) = path.parent() {
			fs::create_dir_all(parent).await?;
		}

		let json = serde_json::to_string(self)?;
		let mut file = File::create(path.as_path()).await?;
		file.write_all(json.as_bytes()).await?;
		Ok(())
	}

	// Read the state from a JSON file
	async fn build_from_store_file() -> io::Result<MvtPullingState> {
		let path = MvtPullingState::get_store_file_path();

		let state = if fs::try_exists(&path).await? {
			let mut file = File::open(path.as_path()).await?;
			let mut json = String::new();
			file.read_to_string(&mut json).await?;
			let state = serde_json::from_str(&json)?;
			state
		} else {
			// Return a default state if the file does not exist
			MvtPullingState::default()
		};
		Ok(state)
	}

	fn get_store_file_path() -> std::path::PathBuf {
		let dot_movement = dot_movement::DotMovement::try_from_env()
			.unwrap_or(dot_movement::DotMovement::new(".movement"));
		bridge_config::get_config_path(&dot_movement).join(PULL_STATE_FILE_NAME)
	}

	fn update_state_with_event(
		&mut self,
		event: &BridgeContractEvent<MovementAddress>,
		sequence_number: u64,
	) {
		//define the state to the next event.
		match event {
			BridgeContractEvent::Initiated(_) => {
				if self.initiated <= sequence_number {
					self.initiated = sequence_number + 1
				}
			}
			BridgeContractEvent::Completed(_) => {
				if self.completed <= sequence_number {
					self.completed = sequence_number + 1
				}
			}
		}
	}

	// If an error occurs during deserialization, the event seq_number must be increase
	// to avoid always the fetch the same event.
	fn update_state_with_error(&mut self, err: &BridgeContractError) {
		match err {
			BridgeContractError::EventDeserializingFail(_, event_type) => match event_type {
				BridgeContractEventType::Initiated => self.initiated += 1,
				BridgeContractEventType::Completed => self.completed += 1,
			},
			_ => (),
		}
	}
}

pub struct MovementMonitoring {
	listener:
		futurempsc::UnboundedReceiver<BridgeContractResult<BridgeContractEvent<MovementAddress>>>,
}

impl BridgeContractMonitoring for MovementMonitoring {
	type Address = MovementAddress;
}

impl MovementMonitoring {
	pub async fn build(
		config: &MovementConfig,
		mut health_check_rx: mpsc::Receiver<oneshot::Sender<bool>>,
	) -> Result<Self, anyhow::Error> {
		// Spawn a task to forward events to the listener channel
		let (mut sender, listener) = futures::channel::mpsc::unbounded::<
			BridgeContractResult<BridgeContractEvent<MovementAddress>>,
		>();

		//read the pull state
		let mut pull_state = MvtPullingState::build_from_store_file().await?;

		tokio::spawn({
			let config = config.clone();
			async move {
				loop {
					//Check if there's a health check request
					match health_check_rx.try_recv() {
						Ok(tx) => {
							if let Err(err) = tx.send(true) {
								tracing::warn!(
									"Mvt Heath check send on oneshot channel failed:{err}"
								);
							}
						}
						Err(mpsc::error::TryRecvError::Empty) => (), //nothing
						Err(err) => {
							tracing::warn!("Check Mvt monitoring loop health channel error: {err}");
						}
					}

					let mut init_event_list = match pool_contract(
						FRAMEWORK_ADDRESS,
						&config.mvt_rpc_connection_url(),
						&pull_state,
						config.rest_connection_timeout_secs,
					)
					.await
					{
						Ok(evs) => evs.into_iter().map(|ev| Ok(ev)).collect(),
						Err(err) => vec![Err(err)],
					};

					//extract event sequence_number and update pull state
					let (event_list, new_pull_state) = init_event_list.drain(..).fold(
						(Vec::new(), pull_state.clone()),
						|(mut events, mut state), event| {
							match event {
								Ok((ev, seq)) => {
									state.update_state_with_event(&ev, seq);
									events.push(Ok(ev));
								}
								Err(err) => {
									state.update_state_with_error(&err);
									events.push(Err(err));
								}
							}
							(events, state)
						},
					);

					for event in event_list {
						if sender.send(event).await.is_err() {
							tracing::error!("Failed to send event to listener channel");
							break;
						}
					}
					pull_state = new_pull_state;

					if let Err(err) = pull_state.save_to_store_file().await {
						tracing::error!("MVT monitoring unable to store the file state because:{err} for state:{pull_state:?}");
					}
					let _ = tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
				}
			}
		});

		Ok(MovementMonitoring { listener })
	}
}

impl Stream for MovementMonitoring {
	type Item = BridgeContractResult<BridgeContractEvent<MovementAddress>>;

	fn poll_next(self: Pin<&mut Self>, cx: &mut std::task::Context) -> Poll<Option<Self::Item>> {
		let this = self.get_mut();
		this.listener.poll_next_unpin(cx)
	}
}

#[derive(Debug, PartialEq, Eq, Clone, Deserialize)]
struct CounterpartyCompletedDetails {
	pub bridge_transfer_id: BridgeTransferId,
	pub initiator: BridgeAddress<Vec<u8>>,
	pub recipient: BridgeAddress<MovementAddress>,
	pub amount: Amount,
	pub nonce: Nonce,
}

async fn pool_contract(
	framework_address: AccountAddress,
	rest_url: &str,
	pull_state: &MvtPullingState,
	timeout_sec: u64,
) -> BridgeContractResult<Vec<(BridgeContractEvent<MovementAddress>, u64)>> {
	let struct_tag = format!(
		"{}::native_bridge::BridgeEvents",
		framework_address.to_string()
	);
	// Get initiated events
	let initiated_events = get_account_events(
		rest_url,
		&framework_address.to_string(),
		&struct_tag,
		"bridge_transfer_initiated_events",
		pull_state.initiated,
		timeout_sec,
	)
	.await?
	.into_iter()
	.map(|e| {
		let data: BridgeEventData = serde_json::from_str(&e.data.to_string())?;
		let transfer_details = BridgeTransferInitiatedDetails::try_from(data)?;
		Ok((BridgeContractEvent::Initiated(transfer_details), e.sequence_number.into()))
	})
	.collect::<Result<Vec<_>>>()
	.map_err(|e| {
		BridgeContractError::EventDeserializingFail(
			format!("MVT bridge_transfer_initiated_events de-serialization error:{}", e),
			BridgeContractEventType::Initiated,
		)
	})?;

	// Get completed events
	let completed_events = get_account_events(
		rest_url,
		&framework_address.to_string(),
		&struct_tag,
		"bridge_transfer_completed_events",
		pull_state.completed,
		timeout_sec,
	)
	.await?
	.into_iter()
	.map(|e| {
		let data: BridgeEventData = serde_json::from_str(&e.data.to_string())?;
		let transfer_details = BridgeTransferCompletedDetails::try_from(data)?;
		Ok((BridgeContractEvent::Completed(transfer_details), e.sequence_number.into()))
	})
	.collect::<Result<Vec<_>>>()
	.map_err(|e| {
		BridgeContractError::EventDeserializingFail(
			format!("MVT bridge_transfer_completed_events de-serialization error:{}", e),
			BridgeContractEventType::Completed,
		)
	})?;

	let total_events = initiated_events
		.into_iter()
		.chain(completed_events.into_iter())
		.collect::<Vec<_>>();
	Ok(total_events)
}

#[derive(Debug, Deserialize)]
pub struct BridgeEventData {
	#[serde(deserialize_with = "deserialize_hex_vec")]
	pub bridge_transfer_id: Vec<u8>,
	#[serde(deserialize_with = "deserialize_hex_vec")]
	pub initiator: Vec<u8>,
	#[serde(deserialize_with = "deserialize_hex_vec")]
	pub recipient: Vec<u8>,
	#[serde(deserialize_with = "deserialize_u128_from_string")]
	pub nonce: u128,
	#[serde(deserialize_with = "deserialize_u64_from_string")]
	pub amount: u64,
}

// Custom deserialization function to convert a hex string to Vec<u8>
fn deserialize_hex_vec<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
where
	D: Deserializer<'de>,
{
	let hex_str: &str = Deserialize::deserialize(deserializer)?;
	let hex_str = if hex_str.starts_with("0x") { &hex_str[2..] } else { &hex_str };
	Vec::from_hex(hex_str).map_err(serde::de::Error::custom)
}

fn deserialize_u128_from_string<'de, D>(deserializer: D) -> Result<u128, D::Error>
where
	D: Deserializer<'de>,
{
	let s: String = Deserialize::deserialize(deserializer)?;
	s.parse::<u128>().map_err(serde::de::Error::custom)
}

fn deserialize_u64_from_string<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
	D: Deserializer<'de>,
{
	let s: String = Deserialize::deserialize(deserializer)?;
	s.parse::<u64>().map_err(serde::de::Error::custom)
}

impl TryFrom<BridgeEventData> for BridgeTransferInitiatedDetails<MovementAddress> {
	type Error = BridgeContractError;

	fn try_from(data: BridgeEventData) -> Result<Self, Self::Error> {
		Ok(BridgeTransferInitiatedDetails {
			bridge_transfer_id: BridgeTransferId(data.bridge_transfer_id.try_into().map_err(
				|e| {
					BridgeContractError::ConversionFailed(format!(
					"MVT BridgeTransferDetails data onchain bridge_transfer_id conversion error error:{:?}",
					e
				))
				},
			)?),
			initiator: BridgeAddress(
				MovementAddress::try_from(data.initiator)
					.map_err(|err| BridgeContractError::OnChainError(err.to_string()))?,
			),
			recipient: BridgeAddress(data.recipient),
			nonce: Nonce(data.nonce),
			amount: Amount(data.amount),
		})
	}
}

impl TryFrom<BridgeEventData> for BridgeTransferCompletedDetails<MovementAddress> {
	type Error = BridgeContractError;

	fn try_from(data: BridgeEventData) -> Result<Self, Self::Error> {
		Ok(BridgeTransferCompletedDetails {
			bridge_transfer_id: BridgeTransferId(data.bridge_transfer_id.try_into().map_err(
				|e| {
					BridgeContractError::ConversionFailed(format!(
					"MVT BridgeTransferDetails data onchain bridge_transfer_id conversion error error:{:?}",
					e
				))
				},
			)?),
			initiator: BridgeAddress(data.initiator),
			recipient: BridgeAddress(
				MovementAddress::try_from(data.recipient)
					.map_err(|err| BridgeContractError::OnChainError(err.to_string()))?,
			),
			nonce: Nonce(data.nonce),
			amount: Amount(data.amount),
		})
	}
}

/// Queries events from a specified account on the Aptos blockchain and returns a list of `VersionedEvent`.
///
/// This function sends a GET request to the provided `rest_url` with the account address, event type, and field name
/// to retrieve events starting from the specified `start_version`.
///
/// # Returns
///
/// - `Result<Vec<VersionedEvent>, BridgeContractError>`: On success, returns a vector of `VersionedEvent`.
///
/// # Example Return
///
/// ```json
/// [
///     {
///         "version": "25",
///         "guid": {
///             "creation_number": "5",
///             "account_address": "0xb07a6a200d595dd4ed39d9b91e3132e6c15735549e9920c585b2beec0ae659b6"
///         },
///         "sequence_number": "0",
///         "type": "0xb07a6a200d595dd4ed39d9b91e3132e6c15735549e9920c585b2beec0ae659b6::atomic_bridge_initiator::BridgeTransferInitiatedEvent",
///         "data": {
///             "amount": "100",
///             "bridge_transfer_id": "0xeaefd189df98d57b8f4619584cff1fd67f2787c664ac8e9761ecfd7a6ae1fa2b",
///             "hash_lock": "0xfb54fb738082d0214980feb4055e779d7d4722cb0809d5fbe79df8117801c3bb",
///             "originator": "0xf90391c81027f03cdea491ed8b36ffaced26b6df208a9b569e5baf2590eb9b16",
///             "recipient": "0x3078313233",
///             "time_lock": "1",
///             "state": 1
///         }
///     }
/// ]
async fn get_account_events(
	rest_url: &str,
	account_address: &str,
	event_type: &str,
	field_name: &str,
	start_version: u64,
	timeout_sec: u64,
) -> Result<Vec<VersionedEvent>, BridgeContractError> {
	let url = format!(
		"{}/v1/accounts/{}/events/{}/{}",
		rest_url, account_address, event_type, field_name
	);

	let client = reqwest::Client::new();

	// Send the GET request
	let response = match tokio::time::timeout(
		tokio::time::Duration::from_secs(timeout_sec),
		client
			.get(&url)
			.query(&[("start", &start_version.to_string()[..]), ("limit", "10")])
			.send(),
	)
	.await
	{
		Ok(res) => res.map_err(|e| {
			BridgeContractError::OnChainError(format!(
				"MVT get_account_events {event_type} / {field_name} get request error:{}",
				e
			))
		})?,
		Err(err) => {
			//sleep a few second before retesting.
			tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
			Err(BridgeContractError::OnChainError(format!(
				"MVT get_account_events {event_type} / {field_name} Rpc entry point timeout:{err}",
			)))?
		}
	};

	if response.status().is_success() {
		let body = response.text().await.map_err(|e| {
			BridgeContractError::OnChainError(format!(
				"MVT get_account_events {event_type} / {field_name} get response content error:{e}",
			))
		})?;
		let json_result = serde_json::from_str(&body);
		match json_result {
			Ok(data) => Ok(data),
			Err(e) => Err(BridgeContractError::OnChainError(format!(
				"MVT get_account_events {event_type} / {field_name} json convertion error:{e} with response body:{body}",
			))),
		}
	} else {
		Err(BridgeContractError::OnChainError(format!(
			"MVT get_account_events {event_type} / {field_name} status error {}",
			response.status()
		)))
	}
}
