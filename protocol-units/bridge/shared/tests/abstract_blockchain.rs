use bridge_shared::types::MovementAddressError;
use bridge_shared::types::{
	Amount, AssetType, BridgeTransferDetails, BridgeTransferId, GenUniqueHash, HashLock,
	HashLockPreImage, InitiatorAddress, LockDetails, RecipientAddress, TimeLock,
};
use futures::StreamExt;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaChaRng;

use test_log::test;

mod shared;

use shared::testing::blockchain::{
	AbstractBlockchain, AbstractBlockchainEvent, CounterpartyCall, InitiatorCall, Transaction,
};

use crate::shared::testing::blockchain::{
	counterparty_contract::SmartContractCounterpartyEvent,
	initiator_contract::SmartContractInitiatorEvent,
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct TestAddress(pub &'static str);

impl From<RecipientAddress<TestAddress>> for TestAddress {
	fn from(value: RecipientAddress<TestAddress>) -> Self {
		value.0.clone()
	}
}

impl From<TestAddress> for Vec<u8> {
	fn from(value: TestAddress) -> Self {
		value.0.as_bytes().to_vec()
	}
}

impl TryFrom<Vec<u8>> for TestAddress {
	type Error = MovementAddressError;

	fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
		Ok(Self(static_str_ops::staticize(
			&String::from_utf8(value)
				.map_err(|err| MovementAddressError::AddressConvertionlError)?,
		)))
	}
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct TestHash(pub &'static str);

impl From<TestAddress> for RecipientAddress<Vec<u8>> {
	fn from(value: TestAddress) -> Self {
		RecipientAddress(value.0.as_bytes().to_vec())
	}
}

impl From<RecipientAddress<Vec<u8>>> for TestAddress {
	fn from(value: RecipientAddress<Vec<u8>>) -> Self {
		Self(static_str_ops::staticize(&String::from_utf8(value.0).expect("Invalid UTF-8")))
	}
}

impl From<HashLockPreImage> for TestHash {
	fn from(_value: HashLockPreImage) -> Self {
		todo!()
	}
}

impl GenUniqueHash for TestHash {
	fn gen_unique_hash<R: Rng>(_rng: &mut R) -> Self {
		TestHash("unique_hash")
	}
}

#[test(tokio::test)]
async fn test_initiate_bridge_transfer() {
	let rng = ChaChaRng::from_seed([0u8; 32]);
	let mut blockchain = AbstractBlockchain::<TestAddress, TestHash, _>::new(rng, "TestBlockchain");

	let mut monitor = blockchain.add_event_listener();

	let initiator_address = InitiatorAddress(TestAddress("initiator"));
	let recipient_address = RecipientAddress::from(TestAddress("recipient"));
	let amount = Amount(AssetType::EthAndWeth((1000, 0)));
	let time_lock = TimeLock(100);
	let hash_lock = HashLock(TestHash("hash_lock"));

	let transaction = Transaction::Initiator(InitiatorCall::InitiateBridgeTransfer(
		initiator_address.clone(),
		recipient_address.clone(),
		amount,
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
		AbstractBlockchainEvent::InitiatorContractEvent(Ok(
			SmartContractInitiatorEvent::InitiatedBridgeTransfer(BridgeTransferDetails {
				bridge_transfer_id: BridgeTransferId(TestHash("unique_hash")),
				initiator_address: initiator_address.clone(),
				recipient_address: recipient_address.clone(),
				amount: amount.clone(),
				time_lock: time_lock.clone(),
				hash_lock: hash_lock.clone(),
				state: 1
			})
		))
	);

	let details = blockchain
		.initiator_contract
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
	let initiator_adress = InitiatorAddress(vec![]);
	let recipient_address = RecipientAddress(TestAddress("recipient"));
	let amount = Amount(AssetType::EthAndWeth((1000, 0)));

	let transaction =
		Transaction::Counterparty::<TestAddress, _>(CounterpartyCall::LockBridgeTransfer(
			bridge_transfer_id.clone(),
			hash_lock.clone(),
			time_lock.clone(),
			initiator_adress.clone(),
			recipient_address.clone(),
			amount,
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
		AbstractBlockchainEvent::CounterpartyContractEvent(Ok(
			SmartContractCounterpartyEvent::LockedBridgeTransfer(LockDetails {
				bridge_transfer_id: bridge_transfer_id.clone(),
				hash_lock: hash_lock.clone(),
				time_lock: time_lock.clone(),
				initiator_address: InitiatorAddress(Vec::new()),
				recipient_address: recipient_address.clone(),
				amount,
			})
		))
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
