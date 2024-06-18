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
	testing::blockchain::{AbstractBlockchain, AbstractBlockchainClient},
	testing::rng::TestRng,
};

use crate::shared::{
	B1Client, B1CounterpartyContractMonitoring, B1InitiatorContractMonitoring, B2Client,
	B2CounterpartyContractMonitoring, B2InitiatorContractMonitoring, BC1Address, BC1Hash,
	BC2Address, BC2Hash,
};

mod shared;

#[test(tokio::test)]
async fn test_bridge_service_integration() {
	let rng1 = TestRng::from_seed([0u8; 32]);
	let rng2 = TestRng::from_seed([1u8; 32]);

	let mut blockchain_1 = AbstractBlockchain::<BC1Address, BC1Hash, _>::new(rng1, "Blockchain1");
	let mut blockchain_2 = AbstractBlockchain::<BC2Address, BC2Hash, _>::new(rng2, "Blockchain2");

	let client_1 = AbstractBlockchainClient::new(
		blockchain_1.connection(),
		TestRng::from_seed([0u8; 32]),
		0.1,
		0.05,
	);

	let monitor_1_initiator =
		B1InitiatorContractMonitoring::build(blockchain_1.add_event_listener());
	let monitor_1_counterparty =
		B1CounterpartyContractMonitoring::build(blockchain_1.add_event_listener());

	let client_2 = AbstractBlockchainClient::new(
		blockchain_2.connection(),
		TestRng::from_seed([1u8; 32]),
		0.1,
		0.05,
	);

	let monitor_2_initiator =
		B2InitiatorContractMonitoring::build(blockchain_2.add_event_listener());

	let monitor_2_counterparty =
		B2CounterpartyContractMonitoring::build(blockchain_2.add_event_listener());

	pub struct B1Service {
		pub initiator_contract: B1Client,
		pub initiator_monitoring: B1InitiatorContractMonitoring,
		pub counterparty_contract: B1Client,
		pub counterparty_monitoring: B1CounterpartyContractMonitoring,
	}

	impl BlockchainService for B1Service {
		type Address = BC1Address;
		type Hash = BC1Hash;

		type InitiatorContract = B1Client;
		type CounterpartyContract = B1Client;
		type InitiatorMonitoring = B1InitiatorContractMonitoring;
		type CounterpartyMonitoring = B1CounterpartyContractMonitoring;

		fn initiator_contract(&self) -> &Self::InitiatorContract {
			&self.initiator_contract
		}

		fn counterparty_contract(&self) -> &Self::CounterpartyContract {
			&self.counterparty_contract
		}

		fn initiator_monitoring(&mut self) -> &mut Self::InitiatorMonitoring {
			&mut self.initiator_monitoring
		}

		fn counterparty_monitoring(&mut self) -> &mut Self::CounterpartyMonitoring {
			&mut self.counterparty_monitoring
		}
	}

	impl Stream for B1Service {
		type Item = BlockchainEvent<
			<Self as BlockchainService>::Address,
			<Self as BlockchainService>::Hash,
		>;

		fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
			let this = self.get_mut();
			this.poll_next_event(cx)
		}
	}

	let blockchain_service_1 = B1Service {
		initiator_contract: B1Client::build(client_1.clone()),
		initiator_monitoring: monitor_1_initiator,
		counterparty_contract: B1Client::build(client_1),
		counterparty_monitoring: monitor_1_counterparty,
	};
	pub struct B2Service {
		pub initiator_contract: B2Client,
		pub initiator_monitoring: B2InitiatorContractMonitoring,
		pub counterparty_contract: B2Client,
		pub counterparty_monitoring: B2CounterpartyContractMonitoring,
	}

	impl BlockchainService for B2Service {
		type Address = BC2Address;
		type Hash = BC2Hash;

		type InitiatorContract = B2Client;
		type CounterpartyContract = B2Client;
		type InitiatorMonitoring = B2InitiatorContractMonitoring;
		type CounterpartyMonitoring = B2CounterpartyContractMonitoring;

		fn initiator_contract(&self) -> &Self::InitiatorContract {
			&self.initiator_contract
		}

		fn counterparty_contract(&self) -> &Self::CounterpartyContract {
			&self.counterparty_contract
		}

		fn initiator_monitoring(&mut self) -> &mut Self::InitiatorMonitoring {
			&mut self.initiator_monitoring
		}

		fn counterparty_monitoring(&mut self) -> &mut Self::CounterpartyMonitoring {
			&mut self.counterparty_monitoring
		}
	}

	impl Stream for B2Service {
		type Item = BlockchainEvent<
			<Self as BlockchainService>::Address,
			<Self as BlockchainService>::Hash,
		>;

		fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
			let this = self.get_mut();
			this.poll_next_event(cx)
		}
	}

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
