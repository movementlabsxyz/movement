use futures::{Stream, StreamExt};
use rand::SeedableRng;
use std::{
	pin::Pin,
	task::{Context, Poll},
};
use test_log::test;

use bridge_shared::{
	blockchain_service::{BlockchainService, ContractEvent},
	bridge_contracts::BridgeContractInitiator,
	bridge_monitoring::BridgeContractInitiatorEvent,
	bridge_service::BridgeService,
	types::{
		Amount, BridgeTransferDetails, BridgeTransferId, HashLock, HashLockPreImage,
		InitiatorAddress, RecipientAddress, TimeLock,
	},
};

use crate::shared::{
	B1Client, B1CounterpartyContractMonitoring, B1InitiatorContractMonitoring, B2Client,
	B2CounterpartyContractMonitoring, B2InitiatorContractMonitoring, BC1Address, BC1Hash,
	BC2Address, BC2Hash,
};

mod shared;

use shared::testing::{
	blockchain::{AbstractBlockchain, AbstractBlockchainClient},
	rng::{RngSeededClone, TestRng},
};

#[test(tokio::test)]
async fn test_bridge_service_integration() {
	let mut rng = TestRng::from_seed([0u8; 32]);

	let mut blockchain_1 =
		AbstractBlockchain::<BC1Address, BC1Hash, _>::new(rng.seeded_clone(), "Blockchain1");
	let mut blockchain_2 =
		AbstractBlockchain::<BC2Address, BC2Hash, _>::new(rng.seeded_clone(), "Blockchain2");

	// Contracts and monitors for blockchain 1
	let client_1 =
		AbstractBlockchainClient::new(blockchain_1.connection(), rng.seeded_clone(), 0.1, 0.05);
	let monitor_1_initiator =
		B1InitiatorContractMonitoring::build(blockchain_1.add_event_listener());
	let monitor_1_counterparty =
		B1CounterpartyContractMonitoring::build(blockchain_1.add_event_listener());

	// Contracts and monitors for blockchain 2
	let client_2 =
		AbstractBlockchainClient::new(blockchain_2.connection(), rng.seeded_clone(), 0.1, 0.05);
	let monitor_2_initiator =
		B2InitiatorContractMonitoring::build(blockchain_2.add_event_listener());
	let monitor_2_counterparty =
		B2CounterpartyContractMonitoring::build(blockchain_2.add_event_listener());

	bridge_shared::struct_blockchain_service!(
		B1Service,
		BC1Address,
		BC1Hash,
		B1Client,
		B1Client,
		B1InitiatorContractMonitoring,
		B1CounterpartyContractMonitoring
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
		B2InitiatorContractMonitoring,
		B2CounterpartyContractMonitoring
	);

	let mut blockchain_2_client = B2Client::build(client_2.clone());
	let blockchain_2_service = B2Service {
		initiator_contract: blockchain_2_client.clone(),
		initiator_monitoring: monitor_2_initiator,
		counterparty_contract: blockchain_2_client.clone(),
		counterparty_monitoring: monitor_2_counterparty,
	};

	let mut bridge_service = BridgeService::new(blockchain_1_service, blockchain_2_service);

	// Initiate a bridge transfer
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

	// let mut cx = Context::from_waker(futures::task::noop_waker_ref());

	tokio::spawn(blockchain_1);
	tokio::spawn(blockchain_2);

	let transfer_initiated_event = bridge_service.next().await.expect("No event");
	let transfer_initiated_event =
		transfer_initiated_event.B1I_ContractEvent().expect("Not a B1I event");
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
	dbg!(&transfer_initiated_event);

	blockchain_2_client
		.complete_bridge_transfer(
			BridgeTransferId(BC2Hash::from("unique_hash")),
			HashLockPreImage(vec![1, 2, 3, 4]),
		)
		.await
		.expect("complete_bridge_transfer failed");

	let _event = bridge_service.next().await;
}
