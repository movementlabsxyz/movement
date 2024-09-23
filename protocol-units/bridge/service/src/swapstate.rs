//Swap states
use bridge_shared::bridge_monitoring::BridgeContractInitiatorEvent;
use bridge_shared::types::BridgeTransferId;
use ethereum_bridge::client::Config as EthConfig;
use ethereum_bridge::client::EthClient;
use ethereum_bridge::event_monitoring::EthInitiatorMonitoring;
use ethereum_bridge::types::EthAddress;
use ethereum_bridge::types::EthHash;
use movement_bridge::client::{Config as MovementConfig, MovementClient};
use movement_bridge::event_monitoring::MovementInitiatorMonitoring;
use movement_bridge::utils::MovementAddress;
use movement_bridge::utils::MovementHash;
use std::collections::HashMap;
use thiserror::Error;
use tokio::select;
use tokio_stream::StreamExt;

pub struct SwapEvent<AI, AR> {
	chain: ChainId,
	kind: SwapEventType<AI, AR>,
	transfer_id: SwapTransferId,
}

#[derive(Debug, Clone, Eq, PartialEq, PartialOrd, Ord, Hash)]
pub enum SwapEventType<AI, AR> {
	LockInitiatorEvent {
		intiator_address: InitiatorAddress<AI>,
		counter_part_address: RecipientAddress<AR>,
		hash_lock: HashLock<[u8; 32]>,
		time_lock: TimeLock,
		amount: u64,
	},
	MintLockDoneEvent,
	SecretEvent(Vec<u8>),
	MintLockFailEvent,
	ReleaseBurnEvent,
	TimeoutEvent,
}

pub struct SwapAction<AI, AR> {
	init_chain: ChainId,
	transfer_id: SwapTransferId,
	kind: SwapActionType,
}

pub enum SwapActionType<AI, AR> {
	MintLockCounterPart(AI, AR),
	SwapLocked,
	ReleaseBurnInitiator,
	WaitThenReleaseBurnInitiator,
	RefundInitiator,
	SwapDone,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, PartialOrd, Ord, Hash)]
pub enum ChainId {
	ONE,
	TWO,
}

//Some conversion method to integrate in the current code.
#[derive(Debug, Clone, Copy, Eq, PartialEq, PartialOrd, Ord, Hash)]
pub struct SwapHash(pub [u8; 32]);

impl From<MovementHash> for SwapHash {
	fn from(hash: MovementHash) -> Self {
		Self(hash.0)
	}
}

impl From<EthHash> for SwapHash {
	fn from(hash: EthHash) -> Self {
		Self(hash.0)
	}
}

impl From<SwapHash> for MovementHash {
	fn from(hash: SwapHash) -> Self {
		Self(hash.0)
	}
}

impl From<SwapHash> for EthHash {
	fn from(hash: SwapHash) -> Self {
		Self(hash.0)
	}
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Ord, Eq, Hash)]
pub struct SwapTransferId(SwapHash);

impl From<BridgeTransferId<MovementHash>> for SwapTransferId {
	fn from(transfer_id: BridgeTransferId<MovementHash>) -> Self {
		Self(transfer_id.inner().clone().into())
	}
}

impl From<BridgeTransferId<EthHash>> for SwapTransferId {
	fn from(transfer_id: BridgeTransferId<EthHash>) -> Self {
		Self(transfer_id.inner().clone().into())
	}
}

impl From<SwapTransferId> for BridgeTransferId<MovementHash> {
	fn from(transfer_id: SwapTransferId) -> Self {
		Self(transfer_id.0.into())
	}
}

impl From<SwapTransferId> for BridgeTransferId<EthHash> {
	fn from(transfer_id: SwapTransferId) -> Self {
		Self(transfer_id.0.into())
	}
}

impl From<(BridgeContractInitiatorEvent<MovementAddress, MovementHash>, ChainId)> for SwapEvent {
	fn from(
		(event, chain): (BridgeContractInitiatorEvent<MovementAddress, MovementHash>, ChainId),
	) -> Self {
		match event {
			BridgeContractInitiatorEvent::Initiated(details) => SwapEvent {
				chain,
				kind: SwapEventType::LockInitiatorEvent {
					initiator_address: detail.initiator_address,
					recipient_address: detail.recipient_address,
					hash_lock: detail.hash_lock,
					time_lock: detail.time_lock,
					amount: detail.amount,
				},
				transfer_id: details.bridge_transfer_id.into(),
			},
			BridgeContractInitiatorEvent::Completed(transfer_id) => SwapEvent {
				chain,
				kind: SwapEventType::ReleaseBurnEvent,
				transfer_id: transfer_id.into(),
			},
			BridgeContractInitiatorEvent::Refunded(transfer_id) => SwapEvent {
				chain,
				kind: SwapEventType::ReleaseBurnEvent,
				transfer_id: transfer_id.into(),
			},
		}
	}
}

impl From<(BridgeContractInitiatorEvent<EthAddress, EthHash>, ChainId)> for SwapEvent {
	fn from((event, chain): (BridgeContractInitiatorEvent<EthAddress, EthHash>, ChainId)) -> Self {
		match event {
			BridgeContractInitiatorEvent::Initiated(details) => SwapEvent {
				chain,
				kind: SwapEventType::LockInitiatorEvent {
					initiator_address: detail.initiator_address,
					recipient_address: detail.recipient_address,
					hash_lock: detail.hash_lock,
					time_lock: detail.time_lock,
					amount: detail.amount,
				},
				transfer_id: details.bridge_transfer_id.into(),
			},
			BridgeContractInitiatorEvent::Completed(transfer_id) => SwapEvent {
				chain,
				kind: SwapEventType::ReleaseBurnEvent,
				transfer_id: transfer_id.into(),
			},
			BridgeContractInitiatorEvent::Refunded(transfer_id) => SwapEvent {
				chain,
				kind: SwapEventType::ReleaseBurnEvent,
				transfer_id: transfer_id.into(),
			},
		}
	}
}

pub async fn run_bridge(eth_ws_url: &str) -> Result<(), anyhow::Error> {
	let mut one_stream = EthInitiatorMonitoring::build(eth_ws_url).await?;

	let eth_config = EthConfig::build_for_test();
	let mut one_client = EthClient::new(eth_config).await?;

	let mvt_config = MovementConfig::build_for_test();
	let two_client = MovementClient::new(mvt_config).await?;

	let mut two_stream = MovementInitiatorMonitoring::build("localhost:8080").await?;

	let mut state_runtime = Runtime::<
		alloy::primitives::Address,
		aptos_sdk::types::account_address::AccountAddress,
	>::new();

	loop {
		select! {
			Some(one_event) = one_stream.next() =>{
				let swap_event : SwapEvent = (one_event, ChainId::ONE).into();
				match state_runtime.process_event(swap_event) {
					Ok(action) => {
						//Execute action
						let fut = process_action(action, one_client, two_client);
						let jh = tokio::spawn(async move {
							fut.await
						});
					},
					Err(err) => tracing::warn!("Received an invalid event: {err}"),
				}
			}
			Some(two_event) = two_stream.next() =>{
				let swap_event : SwapEvent = (two_event, ChainId::TWO).into();
				match state_runtime.process_event(swap_event) {
					Ok(action) => {
						//Execute action
					},
					Err(err) => tracing::warn!("Received an invalid event: {err}"),
				}
			}
		}
	}
}

#[derive(Debug, Error)]
pub enum InvalidEventError {
	#[error("Receive an event with a bad chan id")]
	BadChain,
	#[error("Get an initiate swap event with an existing id")]
	InitAnAlreadyExist,
	#[error("Bad event received")]
	BadEvent,
	#[error("No existing state found for a non init event")]
	StateNotFound,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, PartialOrd, Ord, Hash)]
enum SwapStateType {
	Initialized,
	Locked,
	KnowSecret,
	Done,
}

pub struct SwapState<AI, AR> {
	pub state: SwapStateType,
	pub init_chain: ChainId,
	pub transfer_id: SwapTransferId,
	pub intiator_address: InitiatorAddress<AI>,
	pub counter_part_address: RecipientAddress<AR>,
	pub hash_lock: HashLock<H>,
	pub time_lock: TimeLock,
	pub amount: u64,
}

impl<AI, AR> SwapState<AI, AR> {
	fn validate_event(&self, event: &SwapEvent) -> Result<(), InvalidEventError> {
		match (&event.kind, &self.state) {
			(SwapEventType::LockInitiatorEvent, _) => {
				// already present invalid
				Err(InvalidEventError::InitAnAlreadyExist)
			}
			// Mint event must on on the couter part chain.
			(SwapEventType::MintLockDoneEvent, SwapStateType::Initialized) => (event.chain
				!= self.init_chain)
				.then_some(())
				.ok_or(InvalidEventError::BadChain),
			// Mint event is only applied on Initialized swap state
			(SwapEventType::MintLockDoneEvent, _) => Err(InvalidEventError::BadEvent),
			(SwapEventType::SecretEvent(secret), _) => Ok(()),
			(SwapEventType::MintLockFailEvent, _) => Ok(()),
			(SwapEventType::ReleaseBurnEvent, _) => Ok(()),
			(SwapEventType::TimeoutEvent, _) => Ok(()),
		}
	}
}

struct Runtime<AI, AR> {
	swap_state_map: HashMap<SwapTransferId, SwapState<AI, AR>>,
}

impl<AI, AR> Runtime<AI, AR> {
	pub fn new() -> Self {
		Runtime { swap_state_map: HashMap::new() }
	}

	pub fn process_event(
		&mut self,
		event: SwapEvent,
	) -> Result<SwapAction<AI, AR>, InvalidEventError> {
		self.validate_state(&event)?;

		let state_opt = self.swap_state_map.remove(&event.transfer_id);
		//create swap state if need
		let mut state = if let SwapEventType::LockInitiatorEvent(
			intiator_address,
			counter_part_address,
		) = event
		{
			SwapState {
				state: SwapStateType::Initialized,
				init_chain: event.chain,
				transfer_id: event.transfer_id,
				intiator_address,
				counter_part_address,
			}
		} else {
			//tested before state can be unwrap
			state_opt.unwrap();
		};

		let action_kind = match event.kind {
			SwapEventType::LockInitiatorEvent(intiator_address, counter_part_address) => {
				SwapActionType::MintLockCounterPart(intiator_address, counter_part_address)
			}
			SwapEventType::MintLockDoneEvent => {
				//transition event. The counterpart has been locked
				state.state = SwapStateType::Locked;
				SwapActionType::SwapLocked
			}
			SwapEventType::SecretEvent(secret) => {
				//transition event. Alice reveal the secret
				state.state = SwapStateType::KnowSecret;
				SwapActionType::ReleaseBurnInitiator
			}
			SwapEventType::MintLockFailEvent => {
				//No transition, replay the release.
				SwapActionType::WaitThenReleaseBurnInitiator
			}
			SwapEventType::ReleaseBurnEvent => {
				//transition event. Swap is done
				state.state = SwapStateType::Done;
				SwapActionType::SwapDone
			}
			SwapEventType::TimeoutEvent => {
				//transition event. A timeout occurs, the fund will refund automatically.
				state.state = SwapStateType::Done;
				SwapActionType::SwapDone
			}
		};

		let action = SwapAction {
			init_chain: state.init_chain,
			transfer_id: state.transfer_id,
			kind: action_kind,
		};

		self.swap_state_map.insert(state.transfer_id, state);

		Ok(action)
	}

	fn validate_state(&mut self, event: &SwapEvent<AI, AR>) -> Result<(), InvalidEventError> {
		let swap_state_opt = self.swap_state_map.get(&event.transfer_id);
		//validate the associated swap_state.
		swap_state_opt
			.as_ref()
			//if the state is present validate the event is compatible
			.map(|state| state.validate_event(&event))
			//if not validate the event is SwapEventType::LockInitiatorEvent
			.or_else(|| {
				Some(
					(swap_state_opt.is_none() && event.kind == SwapEventType::LockInitiatorEvent)
						.then_some(())
						.ok_or(InvalidEventError::StateNotFound),
				)
			})
			.transpose()?;
		Ok(())
	}
}

fn process_action<AI, AR>(
	mut action: SwapAction<AI, AR>,
	one_client: &mut EthClient,
	two_client: &mut MovementClient,
) -> Option<impl Future<Output = Result<(), Error>> + Send> {
	match action.kind {
		SwapActionType::MintLockCounterPart(intiator_address, counter_part_address) => {
			let future = if action.init_chain == ChainID::ONE {
				two_client.lock_bridge_transfer(
					action.transfer_id.into(),
					HashLock(MovementHash(hash_lock)),
					TimeLock(time_lock),
					intiator_address,
					counter_part_address,
					Amount((amount)),
				)
			} else {
				one_client.lock_bridge_transfer(
					action.transfer_id.into(),
					HashLock(MovementHash(hash_lock)),
					TimeLock(time_lock),
					intiator_address,
					counter_part_address,
					Amount(AssetType::Moveth(amount)),
					intiator_address,
					counter_part_address,
					HashLock(EthHash(hash_lock)),
					TimeLock(100),
					// value has to be > 0
					Amount(AssetType::EthAndWeth((1, 0))), // Eth
				)
			};
			Some(future)
		}
		SwapActionType::SwapLocked => None,
		SwapActionType::ReleaseBurnInitiator => {
			let future = if action.init_chain == ChainID::ONE {
				two_client.complete_bridge_transfer(action.transfer_id.into(), secret)
			} else {
				one_client.complete_bridge_transfer(action.transfer_id.into(), secret)
			};
			Some(future)
		}
		SwapActionType::WaitThenReleaseBurnInitiator => {
			action.kind = SwapActionType::ReleaseBurnInitiator;
			Some(
				tockio::time::sleep(tokio::time::Duration::from_secs(1))
					.then(process_action(action, one_client, two_client)),
			)
		}
		SwapActionType::RefundInitiator => None,
		SwapActionType::SwapDone => None,
	}
}
