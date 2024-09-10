use crate::types::{
	EthAddress, EthHash, EventName, COUNTERPARTY_ABORTED_SELECT, COUNTERPARTY_COMPLETED_SELECT,
	COUNTERPARTY_LOCKED_SELECT, INITIATOR_COMPLETED_SELECT, INITIATOR_INITIATED_SELECT,
	INITIATOR_REFUNDED_SELECT,
};
use crate::EthChainEvent;
use alloy::dyn_abi::EventExt;
use alloy::eips::BlockNumberOrTag;
use alloy::primitives::{address, LogData};
use alloy::providers::{Provider, ProviderBuilder, RootProvider, WsConnect};
use alloy::rpc::types::{Filter, Log};
use alloy::{
	json_abi::{Event, EventParam},
	pubsub::PubSubFrontend,
};
use bridge_shared::bridge_monitoring::{
	BridgeContractCounterpartyEvent, BridgeContractCounterpartyMonitoring,
};
use bridge_shared::initiator_contract::SmartContractInitiatorEvent;
use bridge_shared::types::{Amount, LockDetails, TimeLock};
use bridge_shared::{
	bridge_monitoring::{BridgeContractInitiatorEvent, BridgeContractInitiatorMonitoring},
	counterparty_contract::SmartContractCounterpartyEvent,
	types::{
		BridgeTransferDetails, BridgeTransferId, HashLock, InitiatorAddress, RecipientAddress,
	},
};
use futures::{channel::mpsc::UnboundedReceiver, Stream, StreamExt};
use std::{pin::Pin, task::Poll};

pub struct EthInitiatorMonitoring<A, H> {
	listener: UnboundedReceiver<EthChainEvent<A, H>>,
	ws: RootProvider<PubSubFrontend>,
}

impl BridgeContractInitiatorMonitoring for EthInitiatorMonitoring<EthAddress, EthHash> {
	type Address = EthAddress;
	type Hash = EthHash;
}

impl EthInitiatorMonitoring<EthAddress, EthHash> {
	pub async fn build(
		rpc_url: &str,
		listener: UnboundedReceiver<EthChainEvent<EthAddress, EthHash>>,
	) -> Result<Self, anyhow::Error> {
		let ws = WsConnect::new(rpc_url);
		let ws = ProviderBuilder::new().on_ws(ws).await?;

		//TODO: this should be an arg
		let initiator_address = address!("f39Fd6e51aad88F6F4ce6aB8827279cffFb92266");
		let filter = Filter::new()
			.address(initiator_address)
			.event("BridgeTransferInitiated(bytes32,address,bytes32,uint256)")
			.event("BridgeTransferCompleted(bytes32,bytes32)")
			.from_block(BlockNumberOrTag::Latest);

		let sub = ws.subscribe_logs(&filter).await?;
		let mut sub_stream = sub.into_stream();

		// Spawn a task to forward events to the listener channel
		let (sender, _) =
			tokio::sync::mpsc::unbounded_channel::<EthChainEvent<EthAddress, EthHash>>();

		tokio::spawn(async move {
			while let Some(log) = sub_stream.next().await {
				let event = decode_initiator_log_data(log)
					.map_err(|e| {
						tracing::error!("Failed to decode log data: {:?}", e);
					})
					.expect("Failed to decode log data");
				let event = EthChainEvent::InitiatorContractEvent(Ok(event.into()));
				if sender.send(event).is_err() {
					tracing::error!("Failed to send event to listener channel");
					break;
				}
			}
		});

		Ok(Self { listener, ws })
	}
}

impl Stream for EthInitiatorMonitoring<EthAddress, EthHash> {
	type Item = BridgeContractInitiatorEvent<
		<Self as BridgeContractInitiatorMonitoring>::Address,
		<Self as BridgeContractInitiatorMonitoring>::Hash,
	>;

	fn poll_next(self: Pin<&mut Self>, cx: &mut std::task::Context) -> Poll<Option<Self::Item>> {
		let this = self.get_mut();
		if let Poll::Ready(Some(EthChainEvent::InitiatorContractEvent(contract_result))) =
			this.listener.poll_next_unpin(cx)
		{
			tracing::trace!(
				"InitiatorContractMonitoring: Received contract event: {:?}",
				contract_result
			);

			// Only listen to the initiator contract events
			match contract_result {
				Ok(contract_event) => match contract_event {
					SmartContractInitiatorEvent::InitiatedBridgeTransfer(details) => {
						return Poll::Ready(Some(BridgeContractInitiatorEvent::Initiated(details)))
					}
					SmartContractInitiatorEvent::CompletedBridgeTransfer(bridge_transfer_id) => {
						return Poll::Ready(Some(BridgeContractInitiatorEvent::Completed(
							bridge_transfer_id,
						)))
					}
					SmartContractInitiatorEvent::RefundedBridgeTransfer(bridge_transfer_id) => {
						return Poll::Ready(Some(BridgeContractInitiatorEvent::Refunded(
							bridge_transfer_id,
						)))
					}
				},
				Err(e) => {
					tracing::error!("Error in contract event: {:?}", e);
				}
			}
		}
		Poll::Pending
	}
}

pub struct EthCounterpartyMonitoring<A, H> {
	listener: UnboundedReceiver<EthChainEvent<A, H>>,
	ws: RootProvider<PubSubFrontend>,
}

impl BridgeContractCounterpartyMonitoring for EthCounterpartyMonitoring<EthAddress, EthHash> {
	type Address = EthAddress;
	type Hash = EthHash;
}

impl Stream for EthCounterpartyMonitoring<EthAddress, EthHash> {
	type Item = BridgeContractCounterpartyEvent<
		<Self as BridgeContractCounterpartyMonitoring>::Address,
		<Self as BridgeContractCounterpartyMonitoring>::Hash,
	>;

	fn poll_next(self: Pin<&mut Self>, cx: &mut std::task::Context) -> Poll<Option<Self::Item>> {
		let this = self.get_mut();
		if let Poll::Ready(Some(EthChainEvent::CounterpartyContractEvent(contract_result))) =
			this.listener.poll_next_unpin(cx)
		{
			tracing::trace!(
				"CounterpartyContractMonitoring: Received contract event: {:?}",
				contract_result
			);

			// Only listen to the counterparty contract events
			match contract_result {
				Ok(contract_event) => match contract_event {
					SmartContractCounterpartyEvent::LockedBridgeTransfer(details) => {
						return Poll::Ready(Some(BridgeContractCounterpartyEvent::Locked(details)))
					}
					SmartContractCounterpartyEvent::CompletedBridgeTransfer(bridge_transfer_id) => {
						return Poll::Ready(Some(BridgeContractCounterpartyEvent::Completed(
							bridge_transfer_id,
						)))
					}
				},
				Err(e) => {
					tracing::error!("Error in contract event: {:?}", e);
				}
			}
		}
		Poll::Pending
	}
}

impl EthCounterpartyMonitoring<EthAddress, EthHash> {
	pub async fn build(
		rpc_url: &str,
		listener: UnboundedReceiver<EthChainEvent<EthAddress, EthHash>>,
	) -> Result<Self, anyhow::Error> {
		let ws = WsConnect::new(rpc_url);
		let ws = ProviderBuilder::new().on_ws(ws).await?;

		//TODO: change this value
		let initiator_address = address!("f39Fd6e51aad88F6F4ce6aB8827279cffFb92266");
		let filter = Filter::new()
			.address(initiator_address)
			.event("BridgeTransferLocked(bytes32,address,uint256,bytes32)")
			.event("BridgeTransferCompleted(bytes32,bytes32)")
			.event("BridgeTransferAborted(bytes32)")
			.from_block(BlockNumberOrTag::Latest);

		let sub = ws.subscribe_logs(&filter).await?;
		let mut sub_stream = sub.into_stream();

		// Spawn a task to forward events to the listener channel
		let (sender, _) =
			tokio::sync::mpsc::unbounded_channel::<EthChainEvent<EthAddress, EthHash>>();

		tokio::spawn(async move {
			while let Some(log) = sub_stream.next().await {
				let event = decode_counterparty_log_data(log)
					.map_err(|e| {
						tracing::error!("Failed to decode log data: {:?}", e);
					})
					.expect("Failed to decode log data");
				let event = EthChainEvent::InitiatorContractEvent(Ok(event.into()));
				if sender.send(event).is_err() {
					tracing::error!("Failed to send event to listener channel");
					break;
				}
			}
		});

		Ok(Self { listener, ws })
	}
}

fn decode_initiator_log_data(
	log: Log,
) -> Result<BridgeContractInitiatorEvent<EthAddress, EthHash>, anyhow::Error> {
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
		.ok_or_else(|| anyhow::anyhow!("Failed to find event"))?;

	let decoded = event.decode_log(&log_data, true).expect("Failed to decode log");

	let coerce_bytes = |(bytes, _): (&[u8], usize)| {
		let mut array = [0u8; 32];
		array.copy_from_slice(bytes);
		array
	};

	if let Some(selector) = decoded.selector {
		match selector {
			INITIATOR_INITIATED_SELECT => {
				let bridge_transfer_id = decoded.indexed[0]
					.as_fixed_bytes()
					.map(coerce_bytes)
					.ok_or_else(|| anyhow::anyhow!("Failed to decode BridgeTransferId"))?;
				let initiator_address = decoded.indexed[1]
					.as_address()
					.map(EthAddress)
					.ok_or_else(|| anyhow::anyhow!("Failed to decode InitiatorAddress"))?;
				let recipient_address = decoded.indexed[2]
					.as_fixed_bytes()
					.map(coerce_bytes)
					.ok_or_else(|| anyhow::anyhow!("Failed to decode RecipientAddress"))?;
				let amount = decoded.indexed[3]
					.as_uint()
					.map(|(u, _)| u.into())
					.ok_or_else(|| anyhow::anyhow!("Failed to decode Amount"))?;
				let hash_lock = decoded.indexed[4]
					.as_fixed_bytes()
					.map(coerce_bytes)
					.ok_or_else(|| anyhow::anyhow!("Failed to decode HashLock"))?;
				let time_lock = decoded.indexed[5]
					.as_uint()
					.map(|(u, _)| u.into())
					.ok_or_else(|| anyhow::anyhow!("Failed to decode TimeLock"))?;
				let state = decoded
					.indexed
					.get(6)
					.and_then(|val| val.as_uint())
					.and_then(|(u, _)| u.try_into().ok()) // Try converting to u8
					.ok_or_else(|| anyhow::anyhow!("Failed to decode state as u8"))?;

				let details: BridgeTransferDetails<EthAddress, EthHash> = BridgeTransferDetails {
					bridge_transfer_id: BridgeTransferId(bridge_transfer_id),
					initiator_address: InitiatorAddress(initiator_address),
					recipient_address: RecipientAddress(recipient_address.to_vec()),
					hash_lock: HashLock(hash_lock),
					time_lock,
					amount,
					state,
				};

				Ok(BridgeContractInitiatorEvent::Initiated(details))
			}
			INITIATOR_COMPLETED_SELECT => {
				let bridge_transfer_id = decoded.indexed[0]
					.as_fixed_bytes()
					.map(coerce_bytes)
					.ok_or_else(|| anyhow::anyhow!("Failed to decode BridgeTransferId"))?;

				Ok(BridgeContractInitiatorEvent::Completed(BridgeTransferId(bridge_transfer_id)))
			}
			INITIATOR_REFUNDED_SELECT => {
				let bridge_transfer_id = decoded.indexed[0]
					.as_fixed_bytes()
					.map(coerce_bytes)
					.ok_or_else(|| anyhow::anyhow!("Failed to decode BridgeTransferId"))?;

				Ok(BridgeContractInitiatorEvent::Refunded(BridgeTransferId(bridge_transfer_id)))
			}
			_ => {
				tracing::error!("Unknown event selector: {:x}", selector);
				Err(anyhow::anyhow!("failed to decode event selector"))
			}
		}
	} else {
		tracing::error!("Failed to decode event selector");
		Err(anyhow::anyhow!("Failed to decode event selector"))
	}
}

fn decode_counterparty_log_data(
	log: Log,
) -> Result<BridgeContractCounterpartyEvent<EthAddress, EthHash>, anyhow::Error> {
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
		.ok_or_else(|| anyhow::anyhow!("Failed to find event"))?;

	let decoded = event.decode_log(&log_data, true).expect("Failed to decode log");

	let coerce_bytes = |(bytes, _): (&[u8], usize)| {
		let mut array = [0u8; 32];
		array.copy_from_slice(bytes);
		array
	};

	if let Some(selector) = decoded.selector {
		match selector {
			COUNTERPARTY_LOCKED_SELECT => {
				let bridge_transfer_id = decoded.indexed[0]
					.as_fixed_bytes()
					.map(coerce_bytes)
					.ok_or_else(|| anyhow::anyhow!("Failed to decode BridgeTransferId"))?;
				let initiator_address = decoded.indexed[1]
					.as_fixed_bytes()
					.map(coerce_bytes)
					.ok_or_else(|| anyhow::anyhow!("Failed to decode InitiatorAddress"))?;
				let recipient_address = decoded.indexed[1]
					.as_address()
					.ok_or_else(|| anyhow::anyhow!("Failed to decode RecipientAddress"))?;
				let amount = decoded.indexed[2]
					.as_uint()
					.map(|(u, _)| u.into())
					.ok_or_else(|| anyhow::anyhow!("Failed to decode Amount"))?;
				let hash_lock = decoded.indexed[3]
					.as_fixed_bytes()
					.map(coerce_bytes)
					.ok_or_else(|| anyhow::anyhow!("Failed to decode HashLock"))?;
				let time_lock = decoded.indexed[4]
					.as_uint()
					.map(|(u, _)| u.into())
					.ok_or_else(|| anyhow::anyhow!("Failed to decode TimeLock"))?;
				Ok(BridgeContractCounterpartyEvent::Locked(LockDetails {
					bridge_transfer_id: BridgeTransferId(bridge_transfer_id),
					initiator_address: InitiatorAddress(initiator_address.to_vec()),
					recipient_address: RecipientAddress(EthAddress(recipient_address)),
					amount: Amount(amount),
					hash_lock: HashLock(hash_lock),
					time_lock: TimeLock(time_lock),
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
				Err(anyhow::anyhow!("failed to decode event selector"))
			}
		}
	} else {
		tracing::error!("Failed to decode event selector");
		Err(anyhow::anyhow!("Failed to decode event selector"))
	}
}
