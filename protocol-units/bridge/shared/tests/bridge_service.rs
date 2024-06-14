use std::task::Context;
use test_log::test;

use bridge_shared::bridge_contracts::BridgeContractInitiator;
use bridge_shared::bridge_service::BridgeService;
use bridge_shared::testing::mocks::MockBlockchainService;
use bridge_shared::types::{Amount, HashLock, InitiatorAddress, RecipientAddress, TimeLock};
use futures::StreamExt;

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
struct BC1Hash(pub &'static str);

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
struct BC2Hash(pub &'static str);

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
struct BC1Address(pub &'static str);

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
struct BC2Address(pub &'static str);

#[test(tokio::test)]
async fn test_bridge_service_integration() {
	let blockchain_service_1 = MockBlockchainService::<BC1Address, BC1Hash>::build();
	let blockchain_service_2 = MockBlockchainService::<BC2Address, BC2Hash>::build();

	let mut bridge_service = BridgeService::new(blockchain_service_1, blockchain_service_2);

	// trigger the initiate_bridge_transfer method
	bridge_service
		.blockchain_1
		.initiator_contract
		.with_next_bridge_transfer_id(BC1Hash("transfer_id"))
		.initiate_bridge_transfer(
			InitiatorAddress(BC1Address("initiator")),
			RecipientAddress(BC1Address("recipient")),
			HashLock(BC1Hash("hash_lock")),
			TimeLock(100),
			Amount(1000),
		)
		.await
		.expect("initiate_bridge_transfer failed");

	let mut cx = Context::from_waker(futures::task::noop_waker_ref());
	let event = bridge_service.poll_next_unpin(&mut cx);
}
