use super::types::{
	EthAddress, EventName, COUNTERPARTY_ABORTED_SELECT, COUNTERPARTY_COMPLETED_SELECT,
	COUNTERPARTY_LOCKED_SELECT, INITIATOR_COMPLETED_SELECT, INITIATOR_INITIATED_SELECT,
	INITIATOR_REFUNDED_SELECT,
};
use crate::chains::bridge_contracts::BridgeContractError;
use crate::chains::bridge_contracts::BridgeContractEvent;
use crate::chains::bridge_contracts::BridgeContractMonitoring;
use crate::chains::bridge_contracts::BridgeContractResult;
use crate::types::LockDetails;
use crate::types::{
	Amount, BridgeAddress, BridgeTransferDetails, BridgeTransferId, HashLock, TimeLock,
};
use alloy::dyn_abi::EventExt;
use alloy::eips::BlockNumberOrTag;
use alloy::json_abi::{Event, EventParam};
use alloy::primitives::{address, LogData};
use alloy::providers::{Provider, ProviderBuilder, WsConnect};
use alloy::rpc::types::{Filter, Log};
use bridge_config::common::eth::EthConfig;
use futures::SinkExt;
use futures::{channel::mpsc::UnboundedReceiver, Stream, StreamExt};
use std::{pin::Pin, task::Poll};
use tokio::select;

pub struct EthMonitoring {
	listener: UnboundedReceiver<BridgeContractResult<BridgeContractEvent<EthAddress>>>,
}

impl BridgeContractMonitoring for EthMonitoring {
	type Address = EthAddress;
}

impl EthMonitoring {
	pub async fn build(config: &EthConfig) -> Result<Self, anyhow::Error> {
		let rpc_url = config.eth_rpc_connection_url();
		let ws = WsConnect::new(rpc_url);
		let ws = ProviderBuilder::new().on_ws(ws).await?;

		// Get initiator contract stream.
		//TODO: this should be an arg
		let initiator_address = address!("f39Fd6e51aad88F6F4ce6aB8827279cffFb92266");
		let filter = Filter::new()
			.address(initiator_address)
			.event("BridgeTransferInitiated(bytes32,address,bytes32,uint256)")
			.event("BridgeTransferCompleted(bytes32,bytes32)")
			.from_block(BlockNumberOrTag::Latest);

		let sub = ws.subscribe_logs(&filter).await?;
		let mut initiator_sub_stream = sub.into_stream();

		// Get counterpart contract stream.
		let initiator_address = address!("f39Fd6e51aad88F6F4ce6aB8827279cffFb92266");
		let filter = Filter::new()
			.address(initiator_address)
			.event("BridgeTransferLocked(bytes32,address,uint256,bytes32)")
			.event("BridgeTransferCompleted(bytes32,bytes32)")
			.event("BridgeTransferAborted(bytes32)")
			.from_block(BlockNumberOrTag::Latest);

		let sub = ws.subscribe_logs(&filter).await?;
		let mut counterpart_sub_stream = sub.into_stream();

		// Spawn a task to forward events to the listener channel
		let (mut sender, listener) = futures::channel::mpsc::unbounded::<
			BridgeContractResult<BridgeContractEvent<EthAddress>>,
		>();

		tokio::spawn(async move {
			loop {
				let event;
				select! {
					Some(initialtor_log) = initiator_sub_stream.next() => {
						event = decode_initiator_log_data(initialtor_log)
					}
					Some(counterpart_log) = counterpart_sub_stream.next() => {
						event = decode_counterparty_log_data(counterpart_log)
					}
				};
				if sender.send(event).await.is_err() {
					tracing::error!("Failed to send event to listener channel");
					break;
				}
			}
		});

		Ok(Self { listener })
	}
}

impl Stream for EthMonitoring {
	type Item = BridgeContractResult<BridgeContractEvent<EthAddress>>;

	fn poll_next(self: Pin<&mut Self>, cx: &mut std::task::Context) -> Poll<Option<Self::Item>> {
		let this = self.get_mut();
		this.listener.poll_next_unpin(cx)
	}
}

fn decode_initiator_log_data(log: Log) -> BridgeContractResult<BridgeContractEvent<EthAddress>> {
	let topics = log.topics().to_owned();
	let log_data =
		LogData::new(topics.clone(), log.data().data.clone()).expect("Failed to create log data");

	// Build the event
	let event = topics
		.iter()
		.find_map(|topic| {
			match *topic {
				INITIATOR_INITIATED_SELECT => Some(Event {
					name: EventName::InitiatorInitiated.as_str().to_string(),
					inputs: EventName::InitiatorInitiated
						.params()
						.iter()
						.map(|p| EventParam {
							ty: p.to_string(),
							name: EventName::InitiatorCompleted.as_str().to_string(),
							indexed: true,
							components: EventName::InitiatorInitiated.params(),
							internal_type: None, // for now
						})
						.collect(),
					anonymous: false,
				}),
				INITIATOR_COMPLETED_SELECT => Some(Event {
					name: EventName::InitiatorCompleted.as_str().to_string(),
					inputs: EventName::InitiatorCompleted
						.params()
						.iter()
						.map(|p| EventParam {
							ty: p.to_string(),
							name: p.name.clone(),
							indexed: true,
							components: EventName::InitiatorCompleted.params(),
							internal_type: None, // for now
						})
						.collect(),
					anonymous: false,
				}),
				INITIATOR_REFUNDED_SELECT => Some(Event {
					name: EventName::InitiatorRefunded.as_str().to_string(),
					inputs: EventName::InitiatorRefunded
						.params()
						.iter()
						.map(|p| EventParam {
							ty: p.to_string(),
							name: p.name.clone(),
							indexed: true,
							components: EventName::InitiatorRefunded.params(),
							internal_type: None, // for now
						})
						.collect(),
					anonymous: false,
				}),
				_ => None,
			}
		})
		.ok_or_else(|| BridgeContractError::OnChainUnknownEvent)?;

	let decoded = event.decode_log(&log_data, true).expect("Failed to decode log");

	let coerce_bytes = |(bytes, _): (&[u8], usize)| {
		let mut array = [0u8; 32];
		array.copy_from_slice(bytes);
		array
	};

	if let Some(selector) = decoded.selector {
		match selector {
			INITIATOR_INITIATED_SELECT => {
				let bridge_transfer_id =
					decoded.indexed[0].as_fixed_bytes().map(coerce_bytes).ok_or_else(|| {
						BridgeContractError::ConversionFailed("BridgeTransferId".to_string())
					})?;
				let initiator_address =
					decoded.indexed[1].as_address().map(EthAddress).ok_or_else(|| {
						BridgeContractError::ConversionFailed("InitiatorAddress".to_string())
					})?;
				let recipient_address =
					decoded.indexed[2].as_fixed_bytes().map(coerce_bytes).ok_or_else(|| {
						BridgeContractError::ConversionFailed("RecipientAddress".to_string())
					})?;
				let amount = decoded.indexed[3]
					.as_uint()
					.map(|(u, _)| u.into())
					.ok_or_else(|| BridgeContractError::ConversionFailed("Amount".to_string()))?;
				let hash_lock = decoded.indexed[4]
					.as_fixed_bytes()
					.map(coerce_bytes)
					.ok_or_else(|| BridgeContractError::ConversionFailed("HashLock".to_string()))?;
				let time_lock = decoded.indexed[5]
					.as_uint()
					.map(|(u, _)| u.into())
					.ok_or_else(|| BridgeContractError::ConversionFailed("TimeLock".to_string()))?;
				let state = decoded
					.indexed
					.get(6)
					.and_then(|val| val.as_uint())
					.and_then(|(u, _)| u.try_into().ok()) // Try converting to u8
					.ok_or_else(|| {
						BridgeContractError::ConversionFailed(
							"Failed to decode state as u8".to_string(),
						)
					})?;

				let details: BridgeTransferDetails<EthAddress> = BridgeTransferDetails {
					bridge_transfer_id: BridgeTransferId(bridge_transfer_id),
					initiator_address: BridgeAddress(initiator_address),
					recipient_address: BridgeAddress(recipient_address.to_vec()),
					hash_lock: HashLock(hash_lock),
					time_lock,
					amount,
					state,
				};

				Ok(BridgeContractEvent::Initiated(details))
			}
			INITIATOR_COMPLETED_SELECT => {
				let bridge_transfer_id =
					decoded.indexed[0].as_fixed_bytes().map(coerce_bytes).ok_or_else(|| {
						BridgeContractError::ConversionFailed(
							"Failed to decode BridgeTransferId".to_string(),
						)
					})?;

				Ok(BridgeContractEvent::InitialtorCompleted(BridgeTransferId(bridge_transfer_id)))
			}
			INITIATOR_REFUNDED_SELECT => {
				let bridge_transfer_id =
					decoded.indexed[0].as_fixed_bytes().map(coerce_bytes).ok_or_else(|| {
						BridgeContractError::ConversionFailed("BridgeTransferId".to_string())
					})?;

				Ok(BridgeContractEvent::Refunded(BridgeTransferId(bridge_transfer_id)))
			}
			_ => {
				tracing::error!("Unknown event selector: {:x}", selector);
				Err(BridgeContractError::ConversionFailed("event selector".to_string()))
			}
		}
	} else {
		tracing::error!("Failed to decode event selector");
		Err(BridgeContractError::ConversionFailed("event selector".to_string()))
	}
}

fn decode_counterparty_log_data(log: Log) -> BridgeContractResult<BridgeContractEvent<EthAddress>> {
	let topics = log.topics().to_owned();
	let log_data =
		LogData::new(topics.clone(), log.data().data.clone()).expect("Failed to create log data");

	// Build the event
	let event = topics
		.iter()
		.find_map(|topic| {
			match *topic {
				COUNTERPARTY_LOCKED_SELECT => Some(Event {
					name: EventName::CounterpartyLocked.as_str().to_string(),
					inputs: EventName::CounterpartyLocked
						.params()
						.iter()
						.map(|p| EventParam {
							ty: p.to_string(),
							name: p.name.clone(),
							indexed: true,
							components: EventName::CounterpartyLocked.params(),
							internal_type: None, // for now
						})
						.collect(),
					anonymous: false,
				}),
				COUNTERPARTY_COMPLETED_SELECT => Some(Event {
					name: EventName::CounterpartyCompleted.as_str().to_string(),
					inputs: EventName::CounterpartyCompleted
						.params()
						.iter()
						.map(|p| EventParam {
							ty: p.to_string(),
							name: p.name.clone(),
							indexed: true,
							components: EventName::CounterpartyCompleted.params(),
							internal_type: None, // for now
						})
						.collect(),
					anonymous: false,
				}),
				COUNTERPARTY_ABORTED_SELECT => Some(Event {
					name: EventName::CounterpartyAborted.as_str().to_string(),
					inputs: EventName::CounterpartyAborted
						.params()
						.iter()
						.map(|p| EventParam {
							ty: p.to_string(),
							name: p.name.clone(),
							indexed: true,
							components: EventName::CounterpartyAborted.params(),
							internal_type: None, // for now
						})
						.collect(),
					anonymous: false,
				}),
				_ => None,
			}
		})
		.ok_or_else(|| BridgeContractError::OnChainUnknownEvent)?;

	let decoded = event.decode_log(&log_data, true).expect("Failed to decode log");

	let coerce_bytes = |(bytes, _): (&[u8], usize)| {
		let mut array = [0u8; 32];
		array.copy_from_slice(bytes);
		array
	};

	if let Some(selector) = decoded.selector {
		match selector {
			//TODO: Not sure all these fields are actually indexed
			COUNTERPARTY_LOCKED_SELECT => {
				let bridge_transfer_id =
					decoded.indexed[0].as_fixed_bytes().map(coerce_bytes).ok_or_else(|| {
						BridgeContractError::ConversionFailed("BridgeTransferId".to_string())
					})?;
				let initiator_address =
					decoded.indexed[1].as_fixed_bytes().map(coerce_bytes).ok_or_else(|| {
						BridgeContractError::ConversionFailed("InitiatorAddress".to_string())
					})?;
				let recipient_address = decoded.indexed[1].as_address().ok_or_else(|| {
					BridgeContractError::ConversionFailed("RecipientAddress".to_string())
				})?;
				let amount = decoded.indexed[2]
					.as_uint()
					.map(|(u, _)| u.into())
					.ok_or_else(|| BridgeContractError::ConversionFailed("Amount".to_string()))?;
				let hash_lock = decoded.indexed[3]
					.as_fixed_bytes()
					.map(coerce_bytes)
					.ok_or_else(|| BridgeContractError::ConversionFailed("HashLock".to_string()))?;
				let time_lock: TimeLock = decoded.indexed[4]
					.as_uint()
					.map(|(u, _)| u.into())
					.ok_or_else(|| BridgeContractError::ConversionFailed("TimeLock".to_string()))?;
				Ok(BridgeContractEvent::Locked(LockDetails {
					bridge_transfer_id: BridgeTransferId(bridge_transfer_id),
					initiator_address: BridgeAddress(initiator_address.to_vec()),
					recipient_address: BridgeAddress(EthAddress(recipient_address)),
					amount: Amount(amount),
					hash_lock: HashLock(hash_lock),
					time_lock,
				}))
			}
			COUNTERPARTY_COMPLETED_SELECT => {
				unimplemented!();
				// let bridge_transfer_id = decoded.indexed[0]
				// 	.as_fixed_bytes()
				// 	.map(coerce_bytes)
				// 	.ok_or_else(|| anyhow::anyhow!("Failed to decode BridgeTransferId"))?;
				// let pre_image = decoded.indexed[1]
				// 	.as_fixed_bytes()
				// 	.map(coerce_bytes)
				// 	.ok_or_else(|| anyhow::anyhow!("Failed to decode PreImage"))?;
				// Ok(BridgeContractCounterpartyEvent::Completed(CounterpartyCompletedDetails {}))
			}
			_ => {
				tracing::error!("Unknown event selector: {:x}", selector);
				Err(BridgeContractError::ConversionFailed("event selector".to_string()))
			}
		}
	} else {
		tracing::error!("Failed to decode event selector");
		Err(BridgeContractError::ConversionFailed("event selector".to_string()))
	}
}
