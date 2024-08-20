use std::collections::HashMap;

use bridge_shared::{
	counterparty_contract::{CounterpartyCall, SmartContractCounterparty},
	initiator_contract::{InitiatorCall, SmartContractInitiator},
	types::{Amount, BridgeAddressType, BridgeHashType, GenUniqueHash, RecipientAddress},
};
use event_types::MovementChainEvent;
use futures::{channel::mpsc, task::AtomicWaker};

pub mod client;
pub mod event_monitoring;
pub mod event_types;
pub mod types;
pub mod utils;

#[derive(Debug)]
pub enum Transaction<A, H> {
	Initiator(InitiatorCall<A, H>),
	Counterparty(CounterpartyCall<A, H>),
}

#[allow(unused)]
pub struct MovementChain<A, H, R> {
	pub name: String,
	pub time: u64,
	pub accounts: HashMap<A, Amount>,
	pub events: Vec<MovementChainEvent<A, H>>,

	pub initiator_contract: SmartContractInitiator<A, H, R>,
	pub counterparty_contract: SmartContractCounterparty<A, H>,

	pub transaction_sender: mpsc::UnboundedSender<Transaction<A, H>>,
	pub transaction_receiver: mpsc::UnboundedReceiver<Transaction<A, H>>,

	pub event_listeners: Vec<mpsc::UnboundedSender<MovementChainEvent<A, H>>>,

	waker: AtomicWaker,

	pub _phantom: std::marker::PhantomData<H>,
}

impl<A, H, R> MovementChain<A, H, R>
where
	A: BridgeAddressType + From<RecipientAddress<A>>,
	H: BridgeHashType + GenUniqueHash,
	R: RngSeededClone,
	H: From<HashLockPreImage>,
{
	pub fn new(mut rng: R, name: impl Into<String>) -> Self {
		let accounts = HashMap::new();
		let events = Vec::new();
		let (event_sender, event_receiver) = mpsc::unbounded();
		let event_listeners = Vec::new();

		Self {
			name: name.into(),
			time: 0,
			accounts,
			events,
			initiator_contract: SmartContractInitiator::new(rng.seeded_clone()),
			counterparty_contract: SmartContractCounterparty::new(),
			transaction_sender: event_sender,
			transaction_receiver: event_receiver,
			event_listeners,
			waker: AtomicWaker::new(),
			_phantom: std::marker::PhantomData,
		}
	}
}
