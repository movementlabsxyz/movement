use crate::chains::bridge_contracts::{
	BridgeContractError, BridgeContractEvent, BridgeContractResult,
};
use crate::types::{
	Amount, AssetType, BridgeAddress, BridgeTransferDetails, BridgeTransferId, HashLock,
	HashLockPreImage, LockDetails,
};
use alloy::primitives::{Address, FixedBytes, Log, LogData, Uint};
use alloy::rpc::client::{ClientBuilder, ReqwestClient};
use alloy::rpc::types::{Filter, FilterBlockOption, FilterSet, RawLog};
use alloy::signers::k256::elliptic_curve::bigint::Uint;
use alloy::sol_types::{SolEvent, SolType};
use bridge_config::common::eth::EthConfig;
use futures::{Stream, StreamExt};
use std::pin::Pin;
use std::task::Poll;
use tokio::select;

use super::client::Config;
use super::types::AtomicBridgeInitiator::BridgeTransferInitiated;
use super::types::{EthAddress, INITIATOR_COMPLETED_SELECT, INITIATOR_INITIATED_SELECT};

type Topics = [FilterSet<FixedBytes<32>>; 4];

pub struct EthMonitoring {
	listener: futures::channel::mpsc::UnboundedReceiver<
		BridgeContractResult<BridgeContractEvent<EthAddress>>,
	>,
}

impl EthMonitoring {
	pub async fn build(&self, config: &EthConfig) -> Result<Self, anyhow::Error> {
		let config: Config = config.try_into()?;
		let alloy_client: ReqwestClient = ClientBuilder::default().http(config.rpc_url);
		let (mut sender, listener) = futures::channel::mpsc::unbounded::<
			BridgeContractResult<BridgeContractEvent<EthAddress>>,
		>();

		// Spawn a task to handle both event streams using `select!`
		tokio::spawn(async move {
			loop {
				let mut init_event_list = match poll_initiator_contract(&alloy_client).await {
					Ok(evs) => evs.into_iter().map(|ev| Ok(ev)).collect(),
					Err(err) => vec![Err(err)],
				};
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

async fn poll_initiator_contract(
	client: &ReqwestClient,
	contract_address: Address,
) -> BridgeContractResult<Vec<BridgeContractEvent<EthAddress>>> {
	let initiated_topics = [
		FilterSet::from(INITIATOR_INITIATED_SELECT), // Topic 0: Event signature (BridgeTransferInitiated)
		FilterSet::default(),                        // Topic 1: No filtering for _bridgeTransferId
		FilterSet::default(),                        // Topic 2: No filtering for _originator
		FilterSet::default(),                        // Topic 3: No filtering for _recipient
	];

	let initiated_filter = Filter {
		//TODO: to replace with correct blockheight range
		block_option: FilterBlockOption::Range { from_block: None, to_block: None },
		address: FilterSet::from(contract_address),
		topics: initiated_topics.clone(),
	};

	let logs: Vec<Log> = client
		.request("eth_getLogs", vec![initiated_filter.clone()])
		.await
		.map_err(|e| BridgeContractError::OnChainError(format!("Failed to fetch logs: {}", e)))?;

	let mut events = Vec::new();

	// Iterate over the logs and decode each one into a BridgeContractEvent
	for log in logs {
		if let Ok(event_data) = decode_initiator_initiated(&log.data, &initiated_topics) {
			let event = BridgeContractEvent::Initiated(event_data);
			events.push(event);
		}
	}

	// let completed_topics = [
	// 	FilterSet::from(INITIATOR_COMPLETED_SELECT), // Topic 0: Event signature (BridgeTransferInitiated)
	// 	FilterSet::default(),                        // Topic 1: No filtering for _bridgeTransferId
	// 	FilterSet::default(),                        // Topic 2: No filtering for _originator
	// 	FilterSet::default(),                        // Topic 3: No filtering for _recipient
	// ];
	//
	let logs: Vec<Log> = client
		.request("eth_getLogs", vec![initiated_filter])
		.await
		.map_err(|e| BridgeContractError::OnChainError(format!("Failed to fetch logs: {}", e)))?;

	for log in logs {
		if let Ok(event_data) = decode_initiator_completed(&log.data) {
			let event = BridgeContractEvent::InitiatorCompleted(event_data);
			events.push(event);
		}
	}

	// If no events were found, return an error
	if events.is_empty() {
		Err(BridgeContractError::OnChainError("No events found".to_string()))
	} else {
		// Otherwise, return the collected events
		Ok(events)
	}
}

// // Fetch logs for the counterpart contract using Alloy client
// async fn fetch_counterpart_logs(
// 	client: &ReqwestClient,
// 	contract_address: Address,
// ) -> BridgeContractResult<BridgeContractEvent<EthAddress>> {
// 	let filter = Filter {
// 		from_block: Some(BlockNumberOrTag::Latest),
// 		to_block: None,
// 		address: Some(vec![contract_address]), // Contract address
// 		topics: None,                          // Add your event signature here if necessary
// 		..Default::default()
// 	};
//
// 	let logs: Vec<Log> = client.request("eth_getLogs", vec![filter]).await?;
// 	for log in logs {
// 		if let Ok(event_data) = LockDetails::decode(&log.data) {
// 			let event = BridgeContractEvent::Locked(event_data);
// 			return Ok(event);
// 		}
// 	}
//
// 	Err(BridgeContractError::OnChainError("No events found".to_string()))
// }

fn decode_initiator_initiated(
	log_data: &LogData,
	topics: &Topics,
) -> BridgeContractResult<BridgeTransferDetails<EthAddress>> {
	let coerce_bytes = |bytes: &[u8; 32]| -> [u8; 32] {
		let mut array = [0u8; 32];
		array.copy_from_slice(bytes);
		array
	};

	let bridge_transfer_id = topics[1]
		.iter()
		.next() // Get the first element (if any)
		.map(|fixed_bytes| BridgeTransferId((*fixed_bytes).into()))
		.ok_or_else(|| BridgeContractError::ConversionFailed("BridgeTransferId".to_string()))?;

	let initiator_address = topics[2]
		.iter()
		.next() // Get the first element (if any)
		.map(|fixed_bytes| EthAddress(Address::from_slice(fixed_bytes.as_ref()))) // Convert to EthAddress
		.ok_or_else(|| BridgeContractError::ConversionFailed("InitiatorAddress".to_string()))?;

	// Decode `recipient_address` from topics (Topic 3)
	let recipient_address = topics[3]
		.iter()
		.next() // Get the first element (if any)
		.map(|fixed_bytes| EthAddress(Address::from_slice(fixed_bytes.as_ref()))) // Convert to EthAddress
		.ok_or_else(|| BridgeContractError::ConversionFailed("RecipientAddress".to_string()))?;

	// Decode non-indexed parameters (data) from `log_data`
	let (amount, hash_lock, time_lock): (Amount, HashLock, TimeLock) = decode(&log_data.data)
		.map_err(|err| {
			BridgeContractError::OnChainError(format!("Failed to decode log data: {}", err))
		})?;

	// Construct the `BridgeTransferDetails` struct
	let details = BridgeTransferDetails {
		bridge_transfer_id,
		initiator_address: BridgeAddress(initiator_address),
		recipient_address,
		hash_lock,
		time_lock,
		amount,
		state: 0, // Set default state or apply custom logic if needed
	};

	Ok(details)
}

fn decode_non_indexed(
	log_data: &[u8],
) -> Result<(Amount, HashLock, TimeLock), BridgeContractError> {
	// Using the generated BridgeTransferInitiated struct from Alloy
	let decoded = BridgeTransferInitiated::abi_decode_data(log_data, true).map_err(|err| {
		BridgeContractError::OnChainError(format!("Failed to decode log data: {}", err))
	})?;

	let (amount, hash_lock, time_lock): (Uint<256, 4>, FixedBytes<32>, Uint<256, 4>) = decoded;

	// Extract the values from the decoded struct
	let amount = Amount(AssetType::Moveth(decoded.amount));
	let hash_lock = decoded.hash_lock;
	let time_lock = decoded.time_lock;
	Ok((amount, hash_lock, time_lock))
}

fn decode_initiator_completed(log_data: &LogData) -> BridgeContractResult<BridgeTransferId> {
	let coerce_bytes = |bytes: &[u8]| -> [u8; 32] {
		let mut array = [0u8; 32];
		array.copy_from_slice(bytes);
		array
	};

	let bridge_transfer_id = coerce_bytes(&log_data);
	Ok(BridgeContractEvent::InitiatorCompleted(BridgeTransferId(bridge_transfer_id)))
}

fn decode_initiator_refunded(
	log_data: Vec<u8>,
) -> BridgeContractResult<BridgeContractEvent<EthAddress>> {
	let coerce_bytes = |bytes: &[u8]| -> [u8; 32] {
		let mut array = [0u8; 32];
		array.copy_from_slice(bytes);
		array
	};

	let bridge_transfer_id = coerce_bytes(&log_data);
	Ok(BridgeContractEvent::Refunded(BridgeTransferId(bridge_transfer_id)))
}
