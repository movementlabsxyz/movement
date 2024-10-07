use crate::chains::bridge_contracts::{
	BridgeContractError, BridgeContractEvent, BridgeContractResult,
};
use crate::types::{
	BridgeAddress, BridgeTransferDetails, BridgeTransferId, HashLock, HashLockPreImage, LockDetails,
};
use alloy::primitives::{Address, FixedBytes, Log, LogData};
use alloy::rpc::client::{ClientBuilder, ReqwestClient};
use alloy::rpc::types::{Filter, FilterBlockOption, FilterSet, RawLog};
use bridge_config::common::eth::EthConfig;
use futures::{Stream, StreamExt};
use std::pin::Pin;
use std::task::Poll;
use tokio::select;

use super::client::Config;
use super::types::{EthAddress, INITIATOR_INITIATED_SELECT};

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
	let topics = [
		FilterSet::from(INITIATOR_INITIATED_SELECT), // Topic 0: Event signature (BridgeTransferInitiated)
		FilterSet::default(),                        // Topic 1: No filtering for _bridgeTransferId
		FilterSet::default(),                        // Topic 2: No filtering for _originator
		FilterSet::default(),                        // Topic 3: No filtering for _recipient
	];

	let filter = Filter {
		//TODO: to replace with correct blockheight range
		block_option: FilterBlockOption::Range { from_block: None, to_block: None },
		address: FilterSet::from(contract_address),
		topics: topics.clone(),
	};
	let logs: Vec<Log> = client
		.request("eth_getLogs", vec![filter])
		.await
		.map_err(|e| BridgeContractError::OnChainError(format!("Failed to fetch logs: {}", e)))?;

	let mut events = Vec::new();

	// Iterate over the logs and decode each one into a BridgeContractEvent
	for log in logs {
		if let Ok(event_data) = decode_initiator_initiated(&log.data, &topics) {
			let event = BridgeContractEvent::Initiated(event_data);
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
	let coerce_bytes = |bytes: &[u8]| -> [u8; 32] {
		let mut array = [0u8; 32];
		array.copy_from_slice(bytes);
		array
	};

	let bridge_transfer_id = topics
		.get(1)
		.map(|t| coerce_bytes(t.as_bytes()))
		.ok_or_else(|| BridgeContractError::ConversionFailed("BridgeTransferId".to_string()))?;

	let initiator_address = topics
		.get(2)
		.map(|t| EthAddress(Address::from_slice(t.as_bytes())))
		.ok_or_else(|| BridgeContractError::ConversionFailed("InitiatorAddress".to_string()))?;

	let recipient_address = topics
		.get(3)
		.map(|t| coerce_bytes(t.as_bytes()))
		.ok_or_else(|| BridgeContractError::ConversionFailed("RecipientAddress".to_string()))?;

	let timelock = topics
		.get(4)
		.as_u64()
		.ok_or_else(|| BridgeContractError::ConversionFailed("TimeLock".to_string()))?;

	let decoded_data: [u8; 32] = coerce_bytes(&log_data);

	let details = BridgeTransferDetails {
		bridge_transfer_id: BridgeTransferId(bridge_transfer_id),
		initiator_address: BridgeAddress(initiator_address),
		recipient_address: BridgeAddress(recipient_address.to_vec()),
		hash_lock: HashLock(decoded_data),
		time_lock: 0, // You'll want to decode this as per your contract structure
		amount: 0,    // You'll want to decode this as per your contract structure
		state: 0,     // Add state decoding logic here if necessary
	};

	Ok(BridgeContractEvent::Initiated(details))
}

fn decode_initiator_completed(
	log_data: Vec<u8>,
) -> BridgeContractResult<BridgeContractEvent<EthAddress>> {
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
