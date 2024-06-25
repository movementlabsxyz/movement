use futures::{Stream, StreamExt};
use rand::SeedableRng;
use std::{
	pin::Pin,
	task::{Context, Poll},
};
use test_log::test;

use bridge_shared::{
	blockchain_service::{BlockchainService, ContractEvent},
	bridge_contracts::{BridgeContractCounterparty, BridgeContractInitiator},
	bridge_monitoring::{BridgeContractCounterpartyEvent, BridgeContractInitiatorEvent},
	bridge_service::BridgeService,
	types::{
		Amount, BridgeTransferDetails, Convert, HashLock, HashLockPreImage, InitiatorAddress,
		LockDetails, RecipientAddress, TimeLock,
	},
};

use crate::shared::{
	B1Client, B2Client, BC1Address, BC1Hash, BC2Address, BC2Hash, CounterpartyContractMonitoring,
	InitiatorContractMonitoring,
};

mod shared;

use shared::testing::{
	blockchain::{AbstractBlockchain, AbstractBlockchainClient},
	rng::{RngSeededClone, TestRng},
};

#[test(tokio::test(flavor = "multi_thread", worker_threads = 4))]
async fn test_bridge_service_integration() {
	let mut rng = TestRng::from_seed([0u8; 32]);

	let mut blockchain_1 =
		AbstractBlockchain::<BC1Address, BC1Hash, _>::new(rng.seeded_clone(), "Blockchain1");
	let mut blockchain_2 =
		AbstractBlockchain::<BC2Address, BC2Hash, _>::new(rng.seeded_clone(), "Blockchain2");

	// Contracts and monitors for blockchain 1
	let client_1 =
		AbstractBlockchainClient::new(blockchain_1.connection(), rng.seeded_clone(), 0.0, 0.00);
	let monitor_1_initiator = InitiatorContractMonitoring::build(blockchain_1.add_event_listener());
	let monitor_1_counterparty =
		CounterpartyContractMonitoring::build(blockchain_1.add_event_listener());

	// Contracts and monitors for blockchain 2
	let client_2 =
		AbstractBlockchainClient::new(blockchain_2.connection(), rng.seeded_clone(), 0.0, 0.00);
	let monitor_2_initiator = InitiatorContractMonitoring::build(blockchain_2.add_event_listener());
	let monitor_2_counterparty =
		CounterpartyContractMonitoring::build(blockchain_2.add_event_listener());

	tokio::spawn(blockchain_1);
	tokio::spawn(blockchain_2);

	bridge_shared::struct_blockchain_service!(
		B1Service,
		BC1Address,
		BC1Hash,
		B1Client,
		B1Client,
		InitiatorContractMonitoring<BC1Address, BC1Hash>,
		CounterpartyContractMonitoring<BC1Address, BC1Hash>
	);

	let mut blockchain_1_client = B1Client::build(client_1.clone());
	let blockchain_1_service = B1Service {
		initiator_contract: blockchain_1_client.clone(),
		initiator_monitoring: monitor_1_initiator,
		counterparty_contract: blockchain_1_client.clone(),
		counterparty_monitoring: monitor_1_counterparty,
	};

	bridge_shared::struct_blockchain_service!(
		B2Service,
		BC2Address,
		BC2Hash,
		B2Client,
		B2Client,
		InitiatorContractMonitoring<BC2Address, BC2Hash>,
		CounterpartyContractMonitoring<BC2Address, BC2Hash>
	);

	let mut blockchain_2_client = B2Client::build(client_2.clone());
	let blockchain_2_service = B2Service {
		initiator_contract: blockchain_2_client.clone(),
		initiator_monitoring: monitor_2_initiator,
		counterparty_contract: blockchain_2_client.clone(),
		counterparty_monitoring: monitor_2_counterparty,
	};

	let mut bridge_service = BridgeService::new(blockchain_1_service, blockchain_2_service);

	// The initiator of the swap triggers a bridge transfer, simultaneously time-locking the assets
	// in the smart contract.
	blockchain_1_client
		.initiate_bridge_transfer(
			InitiatorAddress(BC1Address("initiator")),
			RecipientAddress(BC1Address("recipient")),
			HashLock(BC1Hash::from("hash_lock")),
			TimeLock(100),
			Amount(1000),
		)
		.await
		.expect("initiate_bridge_transfer failed");

	// We expect the bridge to recognize the contract event and emit the appropriate message
	let transfer_initiated_event = bridge_service.next().await.expect("No event");
	let transfer_initiated_event =
		transfer_initiated_event.B1I_ContractEvent().expect("Not a B1I event");
	tracing::debug!(?transfer_initiated_event);
	assert_eq!(
		transfer_initiated_event,
		&BridgeContractInitiatorEvent::Initiated(BridgeTransferDetails {
			bridge_transfer_id: transfer_initiated_event.bridge_transfer_id().clone(),
			initiator_address: InitiatorAddress(BC1Address("initiator")),
			recipient_address: RecipientAddress(BC1Address("recipient")),
			hash_lock: HashLock(BC1Hash::from("hash_lock")),
			time_lock: TimeLock(100),
			amount: Amount(1000)
		})
	);

	// Upon recognizing the event, our bridge server has invoked the counterparty
	// contract on blockchain 2 to initiate asset locking within the smart contract.
	let counterparty_locked_event = bridge_service.next().await.expect("No event");
	let counterparty_locked_event =
		counterparty_locked_event.B2C_ContractEvent().expect("Not a B2C event");
	tracing::debug!(?counterparty_locked_event);
	assert_eq!(
		counterparty_locked_event,
		&BridgeContractCounterpartyEvent::Locked(LockDetails {
			bridge_transfer_id: Convert::convert(transfer_initiated_event.bridge_transfer_id()),
			hash_lock: HashLock(BC2Hash::from("hash_lock")),
			time_lock: TimeLock(100),
			recipient_address: RecipientAddress(BC2Address("recipient")),
			amount: Amount(1000)
		})
	);

	// Once the assets are secured within the counterparty smart contract, the initiator is able
	// to execute the complete bridge transfer by disclosing the secret key required to unlock the assets.
	<B2Client as BridgeContractCounterparty>::complete_bridge_transfer(
		&mut blockchain_2_client,
		Convert::convert(transfer_initiated_event.bridge_transfer_id()),
		HashLockPreImage(b"hash_lock".to_vec()),
	)
	.await
	.expect("complete_bridge_transfer failed");

	// TODO: handle followoing event to complete the swap

	let _event = bridge_service.next().await.expect("No event");
	tracing::debug!(?_event);
}
