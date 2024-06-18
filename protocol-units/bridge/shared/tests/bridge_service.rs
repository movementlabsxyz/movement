use futures::{Stream, StreamExt};
use rand::SeedableRng;
use std::{
	pin::Pin,
	task::{Context, Poll},
};
use test_log::test;

use bridge_shared::{
	blockchain_service::{BlockchainEvent, BlockchainService},
	bridge_service::BridgeService,
	testing::{
		blockchain::{AbstractBlockchain, AbstractBlockchainClient},
		rng::{RngSeededClone, TestRng},
	},
};

use crate::shared::{
	B1Client, B1CounterpartyContractMonitoring, B1InitiatorContractMonitoring, B2Client,
	B2CounterpartyContractMonitoring, B2InitiatorContractMonitoring, BC1Address, BC1Hash,
	BC2Address, BC2Hash,
};

mod shared;

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

	let blockchain_service_1 = B1Service {
		initiator_contract: B1Client::build(client_1.clone()),
		initiator_monitoring: monitor_1_initiator,
		counterparty_contract: B1Client::build(client_1),
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

	let blockchain_service_2 = B2Service {
		initiator_contract: B2Client::build(client_2.clone()),
		initiator_monitoring: monitor_2_initiator,
		counterparty_contract: B2Client::build(client_2),
		counterparty_monitoring: monitor_2_counterparty,
	};

	let mut bridge_service = BridgeService::new(blockchain_service_1, blockchain_service_2);

	let mut cx = Context::from_waker(futures::task::noop_waker_ref());
	let _ = bridge_service.poll_next_unpin(&mut cx);
	let _ = bridge_service.poll_next_unpin(&mut cx);
	let _ = bridge_service.poll_next_unpin(&mut cx);
}
