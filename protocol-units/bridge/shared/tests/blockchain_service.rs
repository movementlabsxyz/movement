use bridge_shared::bridge_monitoring::BridgeContractInitiatorEvent;
use bridge_shared::testing::mocks::MockBlockchainService;
use bridge_shared::types::{
	Amount, BridgeTransferDetails, BridgeTransferId, HashLock, InitiatorAddress, RecipientAddress,
	TimeLock,
};
use bridge_shared::{
	blockchain_service::BlockchainEvent, bridge_contracts::BridgeContractInitiator,
};
use futures::StreamExt;
use std::task::{Context, Poll};

#[tokio::test]
async fn test_bridge_transfer_initiated() {
	let mut blockchain_service = MockBlockchainService::build();

	blockchain_service
		.initiator_contract
		.with_next_bridge_transfer_id("transfer_id")
		.initiate_bridge_transfer(
			InitiatorAddress("initiator"),
			RecipientAddress("recipient"),
			HashLock("hash_lock"),
			TimeLock(100),
			Amount(1000),
		)
		.await
		.expect("initiate_bridge_transfer failed");

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
