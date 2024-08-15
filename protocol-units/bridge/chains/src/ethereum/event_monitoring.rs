use crate::ethereum::event_types::EthChainEvent;
use crate::ethereum::types::{EthAddress, EventName};
use alloy::dyn_abi::EventExt;
use alloy::eips::BlockNumberOrTag;
use alloy::primitives::{address, LogData};
use alloy::providers::{Provider, ProviderBuilder, RootProvider, WsConnect};
use alloy::rpc::types::{Filter, Log};
use alloy::{
	json_abi::{Event, EventParam},
	pubsub::PubSubFrontend,
};
use bridge_shared::initiator_contract::SmartContractInitiatorEvent;
use bridge_shared::{
	bridge_monitoring::{BridgeContractInitiatorEvent, BridgeContractInitiatorMonitoring},
	types::{
		BridgeTransferDetails, BridgeTransferId, HashLock, InitiatorAddress, RecipientAddress,
	},
};
use futures::{channel::mpsc::UnboundedReceiver, Stream, StreamExt};
use std::{pin::Pin, task::Poll};

use crate::ethereum::types::{EthHash, COMPLETED_SELECT, INITIATED_SELECT, REFUNDED_SELECT};

#[allow(unused)]
pub struct EthInitiatorMonitoring<A, H> {
	listener: UnboundedReceiver<EthChainEvent<A, H>>,
	ws: RootProvider<PubSubFrontend>,
}

impl BridgeContractInitiatorMonitoring for EthInitiatorMonitoring<EthAddress, EthHash> {
	type Address = EthAddress;
	type Hash = EthHash;
}

#[allow(dead_code)]
impl EthInitiatorMonitoring<EthAddress, EthHash> {
	async fn run(
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
				let event = decode_log_data(log)
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
				},
				Err(e) => {
					tracing::error!("Error in contract event: {:?}", e);
				}
			}
		}
		Poll::Pending
	}
}

fn decode_log_data(
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
				INITIATED_SELECT => Some(Event {
					name: EventName::Initiated.as_str().to_string(),
					inputs: EventName::Initiated
						.params()
						.iter()
						.map(|p| EventParam {
							ty: p.to_string(),
							name: EventName::Completed.as_str().to_string(),
							indexed: true,
							components: EventName::Initiated.params(),
							internal_type: None, // for now
						})
						.collect(),
					anonymous: false,
				}),
				COMPLETED_SELECT => Some(Event {
					name: EventName::Completed.as_str().to_string(),
					inputs: EventName::Completed
						.params()
						.iter()
						.map(|p| EventParam {
							ty: p.to_string(),
							name: p.name.clone(),
							indexed: true,
							components: EventName::Completed.params(),
							internal_type: None, // for now
						})
						.collect(),
					anonymous: false,
				}),
				REFUNDED_SELECT => Some(Event {
					name: EventName::Refunded.as_str().to_string(),
					inputs: EventName::Refunded
						.params()
						.iter()
						.map(|p| EventParam {
							ty: p.to_string(),
							name: p.name.clone(),
							indexed: true,
							components: EventName::Refunded.params(),
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
			INITIATED_SELECT => {
				let bridge_transfer_id = decoded.indexed[0]
					.as_fixed_bytes()
					.map(coerce_bytes)
					.ok_or_else(|| anyhow::anyhow!("Failed to decode BridgeTransferId"))?;
				let initiator_address = decoded.indexed[1]
					.as_address()
					.map(|a| EthAddress(a))
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

				let details: BridgeTransferDetails<EthAddress, EthHash> = BridgeTransferDetails {
					bridge_transfer_id: BridgeTransferId(bridge_transfer_id),
					initiator_address: InitiatorAddress(initiator_address),
					recipient_address: RecipientAddress(recipient_address.to_vec()),
					hash_lock: HashLock(hash_lock),
					time_lock,
					amount,
				};

				return Ok(BridgeContractInitiatorEvent::Initiated(details));
			}
			COMPLETED_SELECT => {
				let bridge_transfer_id = decoded.indexed[0]
					.as_fixed_bytes()
					.map(coerce_bytes)
					.ok_or_else(|| anyhow::anyhow!("Failed to decode BridgeTransferId"))?;

				// We do nothing with the secret in the event here
				return Ok(BridgeContractInitiatorEvent::Completed(BridgeTransferId(
					bridge_transfer_id,
				)));
			}
			REFUNDED_SELECT => {
				let bridge_transfer_id = decoded.indexed[0]
					.as_fixed_bytes()
					.map(coerce_bytes)
					.ok_or_else(|| anyhow::anyhow!("Failed to decode BridgeTransferId"))?;
				return Ok(BridgeContractInitiatorEvent::Refunded(BridgeTransferId(
					bridge_transfer_id,
				)));
			}
			_ => {
				tracing::error!("Unknown event selector: {:x}", selector);
				return Err(anyhow::anyhow!("failed to devode event selector"));
			}
		};
	} else {
		tracing::error!("Failed to decode event selector");
		return Err(anyhow::anyhow!("Failed to decode event selector"));
	}
}
