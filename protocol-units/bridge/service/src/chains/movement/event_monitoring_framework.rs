use super::client::MovementClient;
use super::utils::MovementAddress;
use crate::chains::bridge_contracts::BridgeContractError;
use crate::chains::bridge_contracts::BridgeContractEvent;
use crate::chains::bridge_contracts::BridgeContractEventType;
use crate::chains::bridge_contracts::BridgeContractMonitoring;
use crate::chains::bridge_contracts::BridgeContractResult;
use crate::types::Amount;
use crate::types::AssetType;
use crate::types::BridgeAddress;
use crate::types::BridgeTransferDetails;
use crate::types::BridgeTransferId;
use crate::types::HashLock;
use crate::types::HashLockPreImage;
use crate::types::LockDetails;
use crate::types::TimeLock;
use anyhow::Result;
use aptos_sdk::rest_client::aptos_api_types::VersionedEvent;
use aptos_sdk::types::account_address::AccountAddress;
use bridge_config::common::movement::MovementConfig;
use futures::channel::mpsc::{self};
use futures::SinkExt;
use futures::Stream;
use futures::StreamExt;
use hex::FromHex;
use serde::Deserialize;
use serde::Deserializer;
use serde::Serialize;
use std::{pin::Pin, task::Poll};
use tokio::fs::{self, File};
use tokio::io::{self, AsyncReadExt, AsyncWriteExt};

const PULL_STATE_FILE_NAME: &str = "pullstate.store";
const FRAMEWORK_ADDRESS: AccountAddress = AccountAddress::new([
	0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1,
]);

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
struct MvtPullingState {
	initiator_init: u64,
	initiator_complete: u64,
	initiator_refund: u64,
	counterpart_lock: u64,
	counterpart_complete: u64,
	counterpart_cancel: u64,
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
				if self.initiator_init <= sequence_number {
					self.initiator_init = sequence_number + 1
				}
			}
			BridgeContractEvent::Locked(_) => {
				if self.counterpart_lock <= sequence_number {
					self.counterpart_lock = sequence_number + 1
				}
			}
			BridgeContractEvent::InitialtorCompleted(_) => {
				if self.initiator_complete <= sequence_number {
					self.initiator_complete = sequence_number + 1
				}
			}
			BridgeContractEvent::CounterPartCompleted(_, _) => {
				if self.counterpart_complete <= sequence_number {
					self.counterpart_complete = sequence_number + 1
				}
			}
			BridgeContractEvent::Cancelled(_) => {
				if self.counterpart_cancel <= sequence_number {
					self.counterpart_cancel = sequence_number + 1
				}
			}
			BridgeContractEvent::Refunded(_) => {
				if self.initiator_refund <= sequence_number {
					self.initiator_refund = sequence_number + 1
				}
			}
		}
	}

	// If an error occurs during deserialization, the event seq_number must be increase
	// to avoid always the fetch the same event.
	fn update_state_with_error(&mut self, err: &BridgeContractError) {
		match err {
			BridgeContractError::EventDeserializingFail(_, event_type) => match event_type {
				BridgeContractEventType::Initiated => self.initiator_init += 1,
				BridgeContractEventType::Locked => self.counterpart_lock += 1,
				BridgeContractEventType::InitialtorCompleted => self.initiator_complete += 1,
				BridgeContractEventType::CounterPartCompleted => self.counterpart_complete += 1,
				BridgeContractEventType::Cancelled => self.counterpart_cancel += 1,
				BridgeContractEventType::Refunded => self.initiator_refund += 1,
			},
			_ => (),
		}
	}
}

pub struct MovementMonitoringFramework {
	listener: mpsc::UnboundedReceiver<BridgeContractResult<BridgeContractEvent<MovementAddress>>>,
}

impl BridgeContractMonitoring for MovementMonitoringFramework {
	type Address = MovementAddress;
}

impl MovementMonitoringFramework {
	pub async fn build(config: &MovementConfig) -> Result<Self, anyhow::Error> {
		// Spawn a task to forward events to the listener channel
		let (mut sender, listener) = futures::channel::mpsc::unbounded::<
			BridgeContractResult<BridgeContractEvent<MovementAddress>>,
		>();

		//read the pull state
		let mut pull_state = MvtPullingState::build_from_store_file().await?;

		tokio::spawn({
			let config = config.clone();
			async move {
				let mvt_client = MovementClient::new(&config).await.unwrap();
				loop {
					let mut init_event_list = match pool_initiator_contract(
						FRAMEWORK_ADDRESS,
						&config.mvt_rpc_connection_url(),
						&pull_state,
					)
					.await
					{
						Ok(evs) => evs.into_iter().map(|ev| Ok(ev)).collect(),
						Err(err) => vec![Err(err)],
					};
					let mut counterpart_event_list = match pool_counterpart_contract(
						FRAMEWORK_ADDRESS,
						&config.mvt_rpc_connection_url(),
						&pull_state,
					)
					.await
					{
						Ok(evs) => evs.into_iter().map(|ev| Ok(ev)).collect(),
						Err(err) => vec![Err(err)],
					};

					//extract event sequence_number and update pull state
					let (event_list, new_pull_state) =
						init_event_list.drain(..).chain(counterpart_event_list.drain(..)).fold(
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

		Ok(MovementMonitoringFramework { listener })
	}
}

impl Stream for MovementMonitoringFramework {
	type Item = BridgeContractResult<BridgeContractEvent<MovementAddress>>;

	fn poll_next(self: Pin<&mut Self>, cx: &mut std::task::Context) -> Poll<Option<Self::Item>> {
		let this = self.get_mut();
		this.listener.poll_next_unpin(cx)
	}
}

#[derive(Debug, PartialEq, Eq, Clone, Deserialize)]
struct CounterpartyCompletedDetails {
	pub bridge_transfer_id: BridgeTransferId,
	pub initiator_address: BridgeAddress<Vec<u8>>,
	pub recipient_address: BridgeAddress<MovementAddress>,
	pub hash_lock: HashLock,
	pub secret: HashLockPreImage,
	pub amount: Amount,
}

async fn pool_initiator_contract(
	native_address: AccountAddress,
	rest_url: &str,
	pull_state: &MvtPullingState,
) -> BridgeContractResult<Vec<(BridgeContractEvent<MovementAddress>, u64)>> {
	let native_address_str = native_address.to_standard_string();
	let struct_tag =
		format!("{}::atomic_bridge_initiator::BridgeTransferInitiatedEvent", native_address_str,);

	// Get initiated events
	let initiated_events = get_account_events(
		rest_url,
		&native_address_str,
		&struct_tag,
		"bridge_transfer_initiated_events",
		pull_state.initiator_init,
	)
	.await?
	.into_iter()
	.map(|e| {
		println!("Initiate event data: {:?} sequence_number:{}", e.data, e.sequence_number);
		let data: BridgeInitEventData = serde_json::from_str(&e.data.to_string())?;
		let transfer_details = BridgeTransferDetails::try_from(data)?;
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
		&native_address_str,
		&struct_tag,
		"bridge_transfer_completed_events",
		pull_state.initiator_complete,
	)
	.await?
	.into_iter()
	.map(|e| {
		println!("complete event data: {:?} sequence_number:{}", e.data, e.sequence_number);
		let data: BridgeCompletEventData = serde_json::from_str(&e.data.to_string())?;
		let event = BridgeContractEvent::InitialtorCompleted(
			data.bridge_transfer_id.try_into().map_err(|err| {
				BridgeContractError::ConversionFailed(format!(
				"MVT initiatorbridge_transfer_completed_events bridge_transfer_id can't be reconstructed:{:?}",
				err
			))
			})?,
		);
		Ok((event, e.sequence_number.into()))
	})
	.collect::<Result<Vec<_>>>()
	.map_err(|e| {
		BridgeContractError::EventDeserializingFail(
			format!("MVT bridge_transfer_completed_events de-serialization error:{}", e),
			BridgeContractEventType::InitialtorCompleted,
		)
	})?;

	// Get refunded events
	let refunded_events = get_account_events(
		rest_url,
		&native_address_str,
		&struct_tag,
		"bridge_transfer_refunded_events",
		pull_state.initiator_refund,
	)
	.await?
	.into_iter()
	.map(|e| {
		println!("refund event data: {:?} sequence_number:{}", e.data, e.sequence_number);
		let data = deserialize_hex_vec(e.data)?;
		let event = BridgeContractEvent::Refunded(data.try_into().map_err(|err| {
			BridgeContractError::ConversionFailed(format!(
				"MVT bridge_transfer_refunded_events bridge_transfer_id can't be reconstructed:{:?}",
				err
			))
		})?);
		Ok((event, e.sequence_number.into()))
	})
	.collect::<Result<Vec<_>>>()
	.map_err(|e| {
		BridgeContractError::EventDeserializingFail(
			format!("MVT bridge_transfer_refunded_events de-serialization error:{}", e),
			BridgeContractEventType::Refunded,
		)
	})?;

	let total_events = initiated_events
		.into_iter()
		.chain(completed_events.into_iter())
		.chain(refunded_events.into_iter())
		.collect::<Vec<_>>();
	Ok(total_events)
}

async fn pool_counterpart_contract(
	native_address: AccountAddress,
	rest_url: &str,
	pull_state: &MvtPullingState,
) -> BridgeContractResult<Vec<(BridgeContractEvent<MovementAddress>, u64)>> {
	let native_address_str = native_address.to_standard_string();
	let struct_tag =
		format!("{}::atomic_bridge_counterparty::BridgeTransferDetails", native_address_str);

	// Get locked events
	let locked_events = get_account_events(
		rest_url,
		&native_address_str,
		&struct_tag,
		"bridge_transfer_locked_events",
		pull_state.counterpart_lock,
	)
	.await?
	.into_iter()
	.map(|e| {
		println!("Lock event data: {:?} sequence_number:{}", e.data, e.sequence_number);
		let data: BridgeInitEventData = serde_json::from_str(&e.data.to_string())?;
		let transfer_details = LockDetails::try_from(data)?;
		Ok((BridgeContractEvent::Locked(transfer_details), e.sequence_number.into()))
	})
	.collect::<Result<Vec<_>>>()
	.map_err(|e| {
		BridgeContractError::EventDeserializingFail(
			format!("MVT bridge_transfer_locked_events de-serialization error:{}", e),
			BridgeContractEventType::Locked,
		)
	})?;

	// Get completed events
	let completed_events = get_account_events(
		rest_url,
		&native_address_str,
		&struct_tag,
		"bridge_transfer_completed_events",
		pull_state.counterpart_complete,
	)
	.await?
	.into_iter()
	.map(|e| {
		println!(
			"Counterpart complete event data: {:?} sequence_number:{}",
			e.data, e.sequence_number
		);
		let data: BridgeCompletEventData = serde_json::from_str(&e.data.to_string())?;
		let event = BridgeContractEvent::CounterPartCompleted(
			data.bridge_transfer_id.try_into().map_err(|err| {
				BridgeContractError::ConversionFailed(format!(
				"MVT counterparty bridge_transfer_completed_events bridge_transfer_id can't be reconstructed:{:?}",
				err
			))
			})?,
			HashLockPreImage(data.pre_image.try_into().map_err(|err| {
				BridgeContractError::ConversionFailed(format!(
				"MVT counterparty bridge_transfer_completed_events pre_image can't be reconstructed:{:?}",
				err
			))
			})?),
		);
		Ok((event, e.sequence_number.into()))
	})
	.collect::<Result<Vec<_>>>()
	.map_err(|e| {
		BridgeContractError::EventDeserializingFail(
			format!(
				"MVT counterpart bridge_transfer_completed_events de-serialization error:{}",
				e
			),
			BridgeContractEventType::CounterPartCompleted,
		)
	})?;

	// Get cancelled events
	let cancelled_events = get_account_events(
		rest_url,
		&native_address_str,
		&struct_tag,
		"bridge_transfer_cancelled_events",
		pull_state.counterpart_cancel,
	)
	.await?
	.into_iter()
	.map(|e| {
		println!("refund event data: {:?} sequence_number:{}", e.data, e.sequence_number);
		let data = deserialize_hex_vec(e.data)?;
		let event = BridgeContractEvent::Cancelled(data.try_into().map_err(|err| {
			BridgeContractError::ConversionFailed(format!(
				"MVT bridge_transfer_cancelled_events bridge_transfer_id can't be reconstructed:{:?}",
				err
			))
		})?);
		Ok((event, e.sequence_number.into()))
	})
	.collect::<Result<Vec<_>>>()
	.map_err(|e| {
		BridgeContractError::EventDeserializingFail(
			format!("MVT bridge_transfer_cancelled_events de-serialization error:{}", e),
			BridgeContractEventType::Cancelled,
		)
	})?;

	let total_events = locked_events
		.into_iter()
		.chain(completed_events.into_iter())
		.chain(cancelled_events.into_iter())
		.collect::<Vec<_>>();
	Ok(total_events)
}

#[derive(Debug, Deserialize)]
pub struct BridgeCompletEventData {
	#[serde(deserialize_with = "deserialize_hex_vec")]
	pub bridge_transfer_id: Vec<u8>,
	#[serde(deserialize_with = "deserialize_hex_vec")]
	pub pre_image: Vec<u8>,
}

#[derive(Debug, Deserialize)]
pub struct BridgeInitEventData {
	#[serde(deserialize_with = "deserialize_hex_vec")]
	pub bridge_transfer_id: Vec<u8>,
	#[serde(deserialize_with = "deserialize_hex_vec")]
	pub initiator: Vec<u8>,
	#[serde(deserialize_with = "deserialize_hex_vec")]
	pub recipient: Vec<u8>,
	#[serde(deserialize_with = "deserialize_hex_vec")]
	pub hash_lock: Vec<u8>,
	#[serde(deserialize_with = "deserialize_u64_from_string")]
	pub time_lock: u64,
	#[serde(deserialize_with = "deserialize_u64_from_string")]
	pub amount: u64,
	#[serde(deserialize_with = "deserialize_u8_from_string")]
	pub state: u8,
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

fn deserialize_u8_from_string<'de, D>(deserializer: D) -> Result<u8, D::Error>
where
	D: Deserializer<'de>,
{
	let s: String = Deserialize::deserialize(deserializer)?;
	s.parse::<u8>().map_err(serde::de::Error::custom)
}

fn deserialize_u64_from_string<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
	D: Deserializer<'de>,
{
	let s: String = Deserialize::deserialize(deserializer)?;
	s.parse::<u64>().map_err(serde::de::Error::custom)
}

impl TryFrom<BridgeInitEventData> for BridgeTransferDetails<MovementAddress> {
	type Error = BridgeContractError;

	fn try_from(data: BridgeInitEventData) -> Result<Self, Self::Error> {
		Ok(BridgeTransferDetails {
			bridge_transfer_id: BridgeTransferId(data.bridge_transfer_id.try_into().map_err(
				|e| {
					BridgeContractError::ConversionFailed(format!(
					"MVT BridgeTransferDetails data onchain bridge_transfer_id conversion error error:{:?}",
					e
				))
				},
			)?),
			initiator_address: BridgeAddress(MovementAddress::from(data.initiator)),
			recipient_address: BridgeAddress(data.recipient),
			hash_lock: HashLock(data.hash_lock.try_into().map_err(|e| {
				BridgeContractError::ConversionFailed(format!(
					"MVT BridgeTransferDetails data onchain hash_lock conversion error error:{:?}",
					e
				))
			})?),
			time_lock: TimeLock(data.time_lock),
			amount: Amount(AssetType::Moveth(data.amount)),
			state: 8,
		})
	}
}

impl TryFrom<BridgeInitEventData> for LockDetails<MovementAddress> {
	type Error = BridgeContractError;

	fn try_from(data: BridgeInitEventData) -> Result<Self, Self::Error> {
		Ok(LockDetails {
			bridge_transfer_id: BridgeTransferId(data.bridge_transfer_id.try_into().map_err(
				|e| {
					BridgeContractError::ConversionFailed(format!(
					"MVT BridgeTransferDetails data onchain bridge_transfer_id conversion error error:{:?}",
					e
				))
				},
			)?),
			initiator_address: BridgeAddress(data.recipient),
			recipient_address: BridgeAddress(MovementAddress::from(data.initiator)),
			hash_lock: HashLock(data.hash_lock.try_into().map_err(|e| {
				BridgeContractError::ConversionFailed(format!(
					"MVT BridgeTransferDetails data onchain hash_lock conversion error error:{:?}",
					e
				))
			})?),
			time_lock: TimeLock(data.time_lock),
			amount: Amount(AssetType::Moveth(data.amount)),
			state: data.state,
		})
	}
}

// Example of return string.
// [
//     {
//         "version": "25",
//         "guid":
//         {
//             "creation_number": "5",
//             "account_address": "0xb07a6a200d595dd4ed39d9b91e3132e6c15735549e9920c585b2beec0ae659b6"
//         },
//         "sequence_number": "0",
//         "type": "0xb07a6a200d595dd4ed39d9b91e3132e6c15735549e9920c585b2beec0ae659b6::atomic_bridge_initiator::BridgeTransferInitiatedEvent",
//         "data":
//         {
//             "amount": "100",
//             "bridge_transfer_id": "0xeaefd189df98d57b8f4619584cff1fd67f2787c664ac8e9761ecfd7a6ae1fa2b",
//             "hash_lock": "0xfb54fb738082d0214980feb4055e779d7d4722cb0809d5fbe79df8117801c3bb",
//             "originator": "0xf90391c81027f03cdea491ed8b36ffaced26b6df208a9b569e5baf2590eb9b16",
//             "recipient": "0x3078313233",
//             "time_lock": "1",
//			   "state": 1
//         }
//     }
// ]
async fn get_account_events(
	rest_url: &str,
	account_address: &str,
	event_type: &str,
	field_name: &str,
	start_version: u64,
) -> Result<Vec<VersionedEvent>, BridgeContractError> {
	let url = format!(
		"{}/v1/accounts/{}/events/{}/{}",
		rest_url, account_address, event_type, field_name
	);

	let client = reqwest::Client::new();

	// Send the GET request
	let response = client
		.get(&url)
		.query(&[("start", &start_version.to_string()[..]), ("limit", "10")])
		.send()
		.await
		.map_err(|e| {
			BridgeContractError::OnChainError(format!(
				"MVT get_account_events get request error: {}",
				e
			))
		})?;

	// Print the raw response body
	let raw_body = response.text().await.map_err(|e| {
		BridgeContractError::OnChainError(format!(
			"MVT get_account_events error while reading response body: {}",
			e
		))
	})?;

	// Print the raw response for debugging purposes
	println!("Raw response body: {}", raw_body);

	// Now attempt to parse the response body as JSON
	let response_json: Vec<VersionedEvent> = serde_json::from_str(&raw_body).map_err(|e| {
		BridgeContractError::OnChainError(format!(
			"MVT get_account_events JSON conversion error: {}",
			e
		))
	})?;

	Ok(response_json)
}
