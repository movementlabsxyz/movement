use std::{fmt::Debug, pin::Pin, task::Poll};

use alloy::pubsub::PubSubFrontend;
use alloy_eips::BlockNumberOrTag;
use alloy_primitives::address;
use alloy_provider::{ProviderBuilder, RootProvider, WsConnect};
use alloy_rpc_types::Filter;
use bridge_shared::{
	bridge_monitoring::{BridgeContractInitiatorEvent, BridgeContractInitiatorMonitoring},
	types::{
		BridgeTransferDetails, BridgeTransferId, CompletedDetails, HashLockPreImage, LockDetails,
	},
};
use futures::{channel::mpsc::UnboundedReceiver, Stream};
use thiserror::Error;

use crate::{SCCResult, SCIResult};

pub struct EthInitiatorMonitoring<A, H> {
	listener: UnboundedReceiver<AbstractBlockainEvent<A, H>>,
	ws: RootProvider<PubSubFrontend>,
}

impl<A: Debug, H: Debug> BridgeContractInitiatorMonitoring for EthInitiatorMonitoring<A, H> {
	type Address = A;
	type Hash = H;
}

impl<A, H> EthInitiatorMonitoring<A, H>
where
	A: Debug + Default + Send + 'static,
	H: Debug + Default + Send + From<[u8; 32]> + 'static,
{
	async fn run(
		rpc_url: &str,
		listener: UnboundedReceiver<AbstractBlockainEvent<A, H>>,
	) -> Result<Self, anyhow::Error> {
		let ws = WsConnect::new(rpc_url);
		let ws = ProviderBuilder::new().on_ws(ws).await?;

		let initiator_address = address!("f39Fd6e51aad88F6F4ce6aB8827279cffFb92266");
		let filter = Filter::new()
			.address(initiator_address)
			.event("BridgeTransferInitiated(bytes32,address,bytes32,uint256)")
			.event("BridgeTransferCompleted(bytes32,bytes32)")
			.from_block(BlockNumberOrTag::Latest);

		let sub = ws.subscribe_logs(&filter).await?;
		let mut sub_stream = sub.into_stream();

		// Spawn a task to forward events to the listener channel
		let (sender, _) = tokio::sync::mpsc::unbounded_channel::<AbstractBlockainEvent<A, H>>();

		tokio::spawn(async move {
			while let Some(log) = sub_stream.next().await {
				let event =
					AbstractBlockainEvent::InitiatorContractEvent(Ok(convert_log_to_event(log)));
				if sender.send(event).is_err() {
					tracing::error!("Failed to send event to listener channel");
					break;
				}
			}
		});

		Ok(Self { listener, ws })
	}
}

impl<A: Debug, H: Debug> Stream for EthInitiatorMonitoring<A, H> {
	type Item = BridgeContractInitiatorEvent<
		<Self as BridgeContractInitiatorMonitoring>::Address,
		<Self as BridgeContractInitiatorMonitoring>::Hash,
	>;

	fn poll_next(self: Pin<&mut Self>, cx: &mut std::task::Context) -> Poll<Option<Self::Item>> {
		let this = self.get_mut();
		if let Poll::Ready(Some(AbstractBlockainEvent::InitiatorContractEvent(contract_result))) =
			this.listener.poll_next_unpin(cx)
		{
			tracing::trace!(
				"InitiatorContractMonitoring: Received contract event: {:?}",
				contract_result
			);

			// Only listen to the initiator contract events
			match contract_result {
				Ok(contract_event) => match contract_event {
					BridgeContractInitiatorEvent::Initiated(details) => {
						return Poll::Ready(Some(BridgeContractInitiatorEvent::Initiated(details)));
					}
					BridgeContractInitiatorEvent::Completed(id) => {
						return Poll::Ready(Some(BridgeContractInitiatorEvent::Completed(id)))
					}
					BridgeContractInitiatorEvent::Refunded(id) => {
						return Poll::Ready(Some(BridgeContractInitiatorEvent::Refunded(id)))
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MoveCounterpartyEvent<H> {
	LockedBridgeTransfer(LockDetails<H>),
	CompletedBridgeTransfer(CompletedDetails<H>),
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum MoveCounterpartyError {
	#[error("Transfer not found")]
	TransferNotFound,
	#[error("Invalid hash lock pre image (secret)")]
	InvalidHashLockPreImage,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EthInitiatorEvent<A, H> {
	InitiatedBridgeTransfer(BridgeTransferDetails<A, H>),
	CompletedBridgeTransfer(BridgeTransferId<H>, HashLockPreImage),
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum EthInitiatorError {
	#[error("Failed to initiate bridge transfer")]
	InitiateTransferError,
	#[error("Transfer not found")]
	TransferNotFound,
	#[error("Invalid hash lock pre image (secret)")]
	InvalidHashLockPreImage,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AbstractBlockainEvent<A, H> {
	InitiatorContractEvent(SCIResult<A, H>),
	CounterpartyContractEvent(SCCResult<H>),
	Noop,
}

// Utility functions
fn convert_log_to_event<A, H>(log: Log) -> BridgeContractInitiatorEvent<A, H>
where
	A: Default,
	H: Default + From<[u8; 32]>,
{
	let initiated_log = AtomicBridgeInitiator::BridgeTransferInitiated::SIGNATURE_HASH;
	let completed_log = AtomicBridgeInitiator::BridgeTransferCompleted::SIGNATURE_HASH;
	let refunded_log = AtomicBridgeInitiator::BridgeTransferRefunded::SIGNATURE_HASH;

	// Extract details from the log and map to event type
	let topics = log.topics();
	let data = log.data().clone();

	// Assuming the first topic is the event type identifier
	let topic = topics.get(0).expect("Expected event type in topics");

	if topic == &initiated_log {
		// Decode the data for Initiated event
		let tokens = decode_log_data(
			&data,
			&[
				ParamType::FixedBytes(32), // bridge_transfer_id
				ParamType::Address,        // initiator_address
				ParamType::Address,        // recipient_address
				ParamType::FixedBytes(32), // hash_lock
				ParamType::Uint(256),      // time_lock
				ParamType::Uint(256),      // amount
			],
		);

		let bridge_transfer_id =
			BridgeTransferId(H::from(tokens[0].clone().into_fixed_bytes().unwrap()));
		let initiator_address = InitiatorAddress(A::from(
			tokens[1].clone().into_address().unwrap().as_fixed_bytes().try_into().unwrap(),
		));
		let recipient_address = RecipientAddress(tokens[2].clone().into_address().unwrap());
		let hash_lock = HashLock(H::from(tokens[3].clone().into_fixed_bytes().unwrap()));
		let time_lock = TimeLock(tokens[4].clone().into_uint().unwrap().as_u64());
		let amount = Amount(tokens[5].clone().into_uint().unwrap());

		let details = BridgeTransferDetails {
			bridge_transfer_id,
			initiator_address,
			recipient_address,
			hash_lock,
			time_lock,
			amount,
		};

		BridgeContractInitiatorEvent::Initiated(details)
	} else if topic == &completed_log {
		// Decode the data for Completed event
		let bridge_transfer_id = BridgeTransferId(H::from(
			topics
				.get(1)
				.expect("Expected hash in topics")
				.as_fixed_bytes()
				.try_into()
				.unwrap(),
		));
		BridgeContractInitiatorEvent::Completed(bridge_transfer_id)
	} else if topic == &refunded_log {
		// Decode the data for Refunded event
		let bridge_transfer_id = BridgeTransferId(H::from(
			topics
				.get(1)
				.expect("Expected hash in topics")
				.as_fixed_bytes()
				.try_into()
				.unwrap(),
		));
		BridgeContractInitiatorEvent::Refunded(bridge_transfer_id)
	} else {
		unimplemented!("Unexpected event type");
	}
}

fn decode_log_data(name: &str, data: &[u8], params: &[ParamType]) -> Vec<Token> {
	let event = Event {
		name: name.to_string(),
		inputs: params
			.iter()
			.map(|p| ethabi::EventParam { name: "".to_string(), kind: p.clone(), indexed: false })
			.collect(),
		anonymous: false,
	};
	let raw_log = RawLog { topics: vec![], data: data.to_vec() };
	event
		.parse_log(raw_log)
		.expect("Unable to parse log data")
		.params
		.into_iter()
		.map(|p| p.value)
		.collect()
}
