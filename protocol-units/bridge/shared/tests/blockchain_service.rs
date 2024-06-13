use bridge_shared::blockchain_service::BlockchainEvent;
use bridge_shared::bridge_monitoring::BridgeContractInitiatorEvent;
use bridge_shared::testing::mocks::{
	MockBlockchainService, MockCounterpartyContract, MockCounterpartyMonitoring,
	MockInitiatorContract, MockInitiatorMonitoring,
};
use bridge_shared::types::{
	Amount, BridgeTransferDetails, BridgeTransferId, HashLock, InitiatorAddress, RecipientAddress,
	TimeLock,
};
use futures::StreamExt;
use std::task::{Context, Poll};

#[tokio::test]
async fn test_bridge_transfer_initiated() {
	let initiator_monitoring = MockInitiatorMonitoring {
		events: vec![BridgeContractInitiatorEvent::BridgeTransferInitiated(
			BridgeTransferDetails {
				bridge_transfer_id: BridgeTransferId("transfer_id"),
				initiator_address: InitiatorAddress("initiator"),
				recipient_address: RecipientAddress("recipient"),
				hash_lock: HashLock("hash_lock"),
				time_lock: TimeLock(100),
				amount: Amount(1000),
			},
		)],
	};

	let counterparty_monitoring = MockCounterpartyMonitoring { events: vec![] };

	let initiator_contract = MockInitiatorContract;
	let counterparty_contract = MockCounterpartyContract;

	let mut blockchain_service = MockBlockchainService {
		initiator_contract,
		initiator_monitoring,
		counterparty_contract,
		counterparty_monitoring,
	};

	let mut cx = Context::from_waker(futures::task::noop_waker_ref());
	let event = blockchain_service.poll_next_unpin(&mut cx);

	assert_eq!(
		event,
		Poll::Ready(Some(BlockchainEvent::InitiatorEvent(
			BridgeContractInitiatorEvent::BridgeTransferInitiated(BridgeTransferDetails {
				bridge_transfer_id: BridgeTransferId("transfer_id"),
				initiator_address: InitiatorAddress("initiator"),
				recipient_address: RecipientAddress("recipient"),
				hash_lock: HashLock("hash_lock"),
				time_lock: TimeLock(100),
				amount: Amount(1000),
			})
		)))
	);
}
