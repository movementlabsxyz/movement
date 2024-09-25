//Swap states
use bridge_shared::bridge_monitoring::BridgeContractInitiatorEvent;
use bridge_shared::types::BridgeTransferId;
use ethereum_bridge::client::Config as EthConfig;
use ethereum_bridge::client::EthClient;
use ethereum_bridge::event_monitoring::EthInitiatorMonitoring;
use ethereum_bridge::types::EthAddress;
use ethereum_bridge::types::EthHash;
use movement_bridge::client::{Config as MovementConfig, MovementClient};
use movement_bridge::event_monitoring::MovementInitiatorMonitoring;
use movement_bridge::utils::MovementAddress;
use movement_bridge::utils::MovementHash;
use std::collections::HashMap;
use thiserror::Error;
use tokio::select;
use tokio_stream::StreamExt;

//Some conversion method to integrate in the current code.
#[derive(Debug, Clone, Copy, Eq, PartialEq, PartialOrd, Ord, Hash)]
pub struct SwapHash(pub [u8; 32]);

impl From<MovementHash> for SwapHash {
	fn from(hash: MovementHash) -> Self {
		Self(hash.0)
	}
}

impl From<EthHash> for SwapHash {
	fn from(hash: EthHash) -> Self {
		Self(hash.0)
	}
}

impl From<SwapHash> for MovementHash {
	fn from(hash: SwapHash) -> Self {
		Self(hash.0)
	}
}

impl From<SwapHash> for EthHash {
	fn from(hash: SwapHash) -> Self {
		Self(hash.0)
	}
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Ord, Eq, Hash)]
pub struct SwapTransferId(SwapHash);

impl From<BridgeTransferId<MovementHash>> for SwapTransferId {
	fn from(transfer_id: BridgeTransferId<MovementHash>) -> Self {
		Self(transfer_id.inner().clone().into())
	}
}

impl From<BridgeTransferId<EthHash>> for SwapTransferId {
	fn from(transfer_id: BridgeTransferId<EthHash>) -> Self {
		Self(transfer_id.inner().clone().into())
	}
}

impl From<SwapTransferId> for BridgeTransferId<MovementHash> {
	fn from(transfer_id: SwapTransferId) -> Self {
		Self(transfer_id.0.into())
	}
}

impl From<SwapTransferId> for BridgeTransferId<EthHash> {
	fn from(transfer_id: SwapTransferId) -> Self {
		Self(transfer_id.0.into())
	}
}

impl From<(BridgeContractInitiatorEvent<MovementAddress, MovementHash>, ChainId)> for SwapEvent {
	fn from(
		(event, chain): (BridgeContractInitiatorEvent<MovementAddress, MovementHash>, ChainId),
	) -> Self {
		match event {
			BridgeContractInitiatorEvent::Initiated(details) => SwapEvent {
				chain,
				kind: SwapEventType::LockInitiatorEvent {
					initiator_address: detail.initiator_address,
					recipient_address: detail.recipient_address,
					hash_lock: detail.hash_lock,
					time_lock: detail.time_lock,
					amount: detail.amount,
				},
				transfer_id: details.bridge_transfer_id.into(),
			},
			BridgeContractInitiatorEvent::Completed(transfer_id) => SwapEvent {
				chain,
				kind: SwapEventType::ReleaseBurnEvent,
				transfer_id: transfer_id.into(),
			},
			BridgeContractInitiatorEvent::Refunded(transfer_id) => SwapEvent {
				chain,
				kind: SwapEventType::ReleaseBurnEvent,
				transfer_id: transfer_id.into(),
			},
		}
	}
}

impl From<(BridgeContractInitiatorEvent<EthAddress, EthHash>, ChainId)> for SwapEvent {
	fn from((event, chain): (BridgeContractInitiatorEvent<EthAddress, EthHash>, ChainId)) -> Self {
		match event {
			BridgeContractInitiatorEvent::Initiated(details) => SwapEvent {
				chain,
				kind: SwapEventType::LockInitiatorEvent {
					initiator_address: detail.initiator_address,
					recipient_address: detail.recipient_address,
					hash_lock: detail.hash_lock,
					time_lock: detail.time_lock,
					amount: detail.amount,
				},
				transfer_id: details.bridge_transfer_id.into(),
			},
			BridgeContractInitiatorEvent::Completed(transfer_id) => SwapEvent {
				chain,
				kind: SwapEventType::ReleaseBurnEvent,
				transfer_id: transfer_id.into(),
			},
			BridgeContractInitiatorEvent::Refunded(transfer_id) => SwapEvent {
				chain,
				kind: SwapEventType::ReleaseBurnEvent,
				transfer_id: transfer_id.into(),
			},
		}
	}
}
