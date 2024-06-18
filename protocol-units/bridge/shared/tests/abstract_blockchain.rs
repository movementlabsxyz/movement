use bridge_shared::types::{
	Amount, BridgeTransferDetails, BridgeTransferId, GenUniqueHash, HashLock, InitiatorAddress,
	RecipientAddress, TimeLock,
};
use bridge_shared::{
	testing::blockchain::{
		AbstractBlockchain, AbstractBlockchainEvent, CounterpartyCall, InitiatorCall, Transaction,
	},
	types::LockedAssetsDetails,
};
use futures::StreamExt;
use rand::SeedableRng;
use rand_chacha::ChaChaRng;

use test_log::test;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct TestAddress(pub &'static str);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct TestHash(pub &'static str);

impl GenUniqueHash for TestHash {
	fn gen_unique_hash() -> Self {
		TestHash("unique_hash")
	}
}

#[test(tokio::test)]
async fn test_initiate_bridge_transfer() {
	let rng = ChaChaRng::from_seed([0u8; 32]);
	let mut blockchain = AbstractBlockchain::<TestAddress, TestHash, _>::new(rng, "TestBlockchain");

	let mut monitor = blockchain.add_event_listener();

	let initiator_address = InitiatorAddress(TestAddress("initiator"));
	let recipient_address = RecipientAddress(TestAddress("recipient"));
	let amount = Amount(1000);
	let time_lock = TimeLock(100);
	let hash_lock = HashLock(TestHash("hash_lock"));

	let transaction = Transaction::Initiator(InitiatorCall::InitiateBridgeTransfer(
		initiator_address.clone(),
		recipient_address.clone(),
		amount.clone(),
		time_lock.clone(),
		hash_lock.clone(),
	));

	blockchain.transaction_sender.unbounded_send(transaction).unwrap();

	let event = blockchain.next().await;
	let monitor_event = monitor.next().await;
	assert!(event.is_some());
	assert!(monitor_event.is_some());
	assert_eq!(event, monitor_event);

	let event = event.unwrap();
	assert_eq!(
		event,
		AbstractBlockchainEvent::BridgeTransferInitiated(BridgeTransferDetails {
			bridge_transfer_id: BridgeTransferId(TestHash("unique_hash")),
			initiator_address: initiator_address.clone(),
			recipient_address: recipient_address.clone(),
			amount: amount.clone(),
			time_lock: time_lock.clone(),
			hash_lock: hash_lock.clone(),
		})
	);

	let details = blockchain
		.initiater_contract
		.initiated_transfers
		.get(&BridgeTransferId(TestHash("unique_hash")));
	assert!(details.is_some());

	let details = details.unwrap();
	assert_eq!(details.initiator_address, initiator_address);
	assert_eq!(details.recipient_address, recipient_address);
	assert_eq!(details.amount, amount);
	assert_eq!(details.time_lock, time_lock);
	assert_eq!(details.hash_lock, hash_lock);
}

#[test(tokio::test)]
async fn test_lock_bridge_transfer() {
	let rng = ChaChaRng::from_seed([0u8; 32]);
	let mut blockchain = AbstractBlockchain::<TestAddress, TestHash, _>::new(rng, "TestBlockchain");

	let mut monitor = blockchain.add_event_listener();

	let bridge_transfer_id = BridgeTransferId(TestHash("unique_hash"));
	let hash_lock = HashLock(TestHash("hash_lock"));
	let time_lock = TimeLock(100);
	let recipient_address = RecipientAddress(TestAddress("recipient"));
	let amount = Amount(1000);

	let transaction = Transaction::Counterparty(CounterpartyCall::LockBridgeTransfer(
		bridge_transfer_id.clone(),
		hash_lock.clone(),
		time_lock.clone(),
		recipient_address.clone(),
		amount.clone(),
	));

	blockchain.transaction_sender.unbounded_send(transaction).unwrap();

	let event = blockchain.next().await;
	let monitor_event = monitor.next().await;
	assert!(monitor_event.is_some());
	assert!(event.is_some());
	assert_eq!(event, monitor_event);

	let event = event.unwrap();
	assert_eq!(
		event,
		AbstractBlockchainEvent::BridgeTransferAssetsLocked(LockedAssetsDetails {
			bridge_transfer_id: bridge_transfer_id.clone(),
			hash_lock: hash_lock.clone(),
			time_lock: time_lock.clone(),
			recipient_address: recipient_address.clone(),
			amount: amount.clone(),
		},)
	);

	let details = blockchain.counterparty_contract.locked_transfers.get(&bridge_transfer_id);
	assert!(details.is_some());

	let details = details.unwrap();
	assert_eq!(details.bridge_transfer_id, bridge_transfer_id);
	assert_eq!(details.recipient_address, recipient_address);
	assert_eq!(details.hash_lock, hash_lock);
	assert_eq!(details.time_lock, time_lock);
	assert_eq!(details.amount, amount);
}
