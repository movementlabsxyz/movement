use futures::{Stream, StreamExt};
use std::pin::Pin;
use std::task::{Context, Poll};

use bridge_shared::bridge_contracts::{
	BridgeContractCounterparty, BridgeContractInitiator, BridgeContractResult,
};
use bridge_shared::bridge_monitoring::{
	BridgeContractCounterpartyEvent, BridgeContractCounterpartyMonitoring,
	BridgeContractInitiatorEvent, BridgeContractInitiatorMonitoring,
};
use bridge_shared::bridge_service::BlockchainService;
use bridge_shared::types::{BridgeTransferDetails, BridgeTransferId};

struct MockInitiatorMonitoring {
	events: Vec<BridgeContractInitiatorEvent<String, String>>,
}

impl Stream for MockInitiatorMonitoring {
	type Item = BridgeContractInitiatorEvent<String, String>;

	fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
		let this = self.get_mut();
		if let Some(event) = this.events.pop() {
			Poll::Ready(Some(event))
		} else {
			Poll::Pending
		}
	}
}

impl BridgeContractInitiatorMonitoring<String, String> for MockInitiatorMonitoring {}

struct MockCounterpartyMonitoring;

impl Stream for MockCounterpartyMonitoring {
	type Item = BridgeContractCounterpartyEvent<String, String>;

	fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
		Poll::Pending
	}
}

impl BridgeContractCounterpartyMonitoring<String, String> for MockCounterpartyMonitoring {}

struct MockInitiatorContract;

#[async_trait::async_trait]
impl BridgeContractInitiator<String, String> for MockInitiatorContract {
	async fn initiate_bridge_transfer(
		&self,
		_initiator_address: String,
		_recipient_address: String,
		_hash_lock: String,
		_time_lock: u64,
		_amount: u64,
	) -> BridgeContractResult<()> {
		Ok(())
	}

	async fn complete_bridge_transfer<S: Send>(
		&self,
		_bridge_transfer_id: BridgeTransferId<String>,
		_secret: S,
	) -> BridgeContractResult<()> {
		Ok(())
	}

	async fn refund_bridge_transfer(
		&self,
		_bridge_transfer_id: BridgeTransferId<String>,
	) -> BridgeContractResult<()> {
		Ok(())
	}

	async fn get_bridge_transfer_details(
		&self,
		_bridge_transfer_id: BridgeTransferId<String>,
	) -> BridgeContractResult<Option<BridgeTransferDetails<String, String>>> {
		Ok(None)
	}
}

struct MockCounterpartyContract;

#[async_trait::async_trait]
impl BridgeContractCounterparty<String, String> for MockCounterpartyContract {
	async fn lock_bridge_transfer_assets(
		&self,
		_bridge_transfer_id: String,
		_hash_lock: String,
		_time_lock: u64,
		_recipient: String,
		_amount: u64,
	) -> bool {
		true
	}

	async fn complete_bridge_transfer<S: Send>(
		&self,
		_bridge_transfer_id: String,
		_secret: S,
	) -> BridgeContractResult<()> {
		Ok(())
	}

	async fn abort_bridge_transfer(
		&self,
		_bridge_transfer_id: BridgeTransferId<String>,
	) -> BridgeContractResult<()> {
		Ok(())
	}

	async fn get_bridge_transfer_details(
		&self,
		_bridge_transfer_id: String,
	) -> BridgeContractResult<Option<BridgeTransferDetails<String, String>>> {
		Ok(None)
	}
}

#[tokio::test]
async fn test_bridge_transfer_initiated() {
	let initiator_monitoring = MockInitiatorMonitoring {
		events: vec![BridgeContractInitiatorEvent::BridgeTransferInitiated(
			BridgeTransferDetails {
				bridge_transfer_id: BridgeTransferId("transfer_id".to_string()),
				initiator_address: "initiator".to_string(),
				recipient_address: "recipient".to_string(),
				hash_lock: "hash_lock".to_string(),
				time_lock: 100,
				amount: 1000,
			},
		)],
	};

	let counter_party_monitoring = MockCounterpartyMonitoring;
	let initiator_contract = MockInitiatorContract;
	let counter_party_contract = MockCounterpartyContract;

	let mut service = BlockchainService::build(
		initiator_contract,
		initiator_monitoring,
		counter_party_contract,
		counter_party_monitoring,
	);

	let mut cx = Context::from_waker(futures::task::noop_waker_ref());
	let event = service.poll_next_unpin(&mut cx);
}
