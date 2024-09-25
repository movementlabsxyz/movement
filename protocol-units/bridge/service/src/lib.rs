use crate::actions::process_action;
use crate::actions::TransferAction;
use crate::chains::bridge_contracts::BridgeContractEvent;
use crate::chains::ethereum::client::{Config as EthConfig, EthClient};
use crate::chains::ethereum::event_monitoring::EthMonitoring;
use crate::chains::ethereum::types::EthAddress;
use crate::chains::movement::client::{Config as MovementConfig, MovementClient};
use crate::chains::movement::event_monitoring::MovementMonitoring;
use crate::chains::movement::utils::MovementAddress;
use crate::events::InvalidEventError;
use crate::events::TransferEvent;
use crate::states::TransferState;
use crate::states::TransferStateType;
use crate::types::BridgeTransferId;
use crate::types::ChainId;
use std::collections::HashMap;
use tokio::select;
use tokio_stream::StreamExt;

mod actions;
mod chains;
mod events;
mod states;
mod types;

pub async fn run_bridge(eth_ws_url: &str) -> Result<(), anyhow::Error> {
	let mut one_stream = EthMonitoring::build(eth_ws_url).await?;

	let eth_config = EthConfig::build_for_test();
	let mut one_client = EthClient::new(eth_config).await?;

	let mvt_config = MovementConfig::build_for_test();
	let two_client = MovementClient::new(&mvt_config).await?;

	let mut two_stream = MovementMonitoring::build(mvt_config).await?;

	let mut state_runtime = Runtime::<
		alloy::primitives::Address,
		aptos_sdk::types::account_address::AccountAddress,
	>::new();

	loop {
		select! {
			Some(one_event_res) = one_stream.next() =>{
				match one_event_res {
					Ok(one_event) => {
						let event : TransferEvent<EthAddress> = (one_event, ChainId::ONE).into();
						match state_runtime.process_event(event) {
							Ok(action) => {
								//Execute action
								let fut = process_action(action, one_client);
								if let Some(fut) = fut {
									let jh = tokio::spawn(fut);
								}

							},
							Err(err) => tracing::warn!("Received an invalid event: {err}"),
						}
					}
					Err(err) => tracing::error!("Chain one event stream return an error:{err}"),
				}
			}
			Some(two_event_res) = two_stream.next() =>{
				match two_event_res {
					Ok(two_event) => {
						let event : TransferEvent<MovementAddress> = (two_event, ChainId::TWO).into();
						match state_runtime.process_event(event) {
							Ok(action) => {
								//Execute action
								let fut = process_action(action, two_client);
								if let Some(fut) = fut {
									let jh = tokio::spawn(fut);
								}

							},
							Err(err) => tracing::warn!("Received an invalid event: {err}"),
						}
					}
					Err(err) => tracing::error!("Chain two event stream return an error:{err}"),
				}
			}
		}
	}
}

struct Runtime {
	swap_state_map: HashMap<BridgeTransferId, TransferState>,
}

impl Runtime {
	pub fn new() -> Self {
		Runtime { swap_state_map: HashMap::new() }
	}

	pub fn process_event<A: From<Vec<u8>>>(
		&mut self,
		event: TransferEvent<A>,
	) -> Result<TransferAction<A>, InvalidEventError> {
		self.validate_state(&event)?;
		let event_transfer_id = event.contract_event.bridge_transfer_id();
		let state_opt = self.swap_state_map.remove(&event_transfer_id);
		//create swap state if need
		let mut state = if let BridgeContractEvent::Initiated(detail) = event.contract_event {
			TransferState {
				state: TransferStateType::Initialized,
				init_chain: event.chain,
				transfer_id: event_transfer_id,
				intiator_address: detail.initiator_address.into(),
				counter_part_address: detail.recipient_address,
				hash_lock: detail.hash_lock,
				time_lock: detail.time_lock,
				amount: detail.amount,
				contract_state: detail.state,
			}
		} else {
			//tested before state can be unwrap
			state_opt.unwrap()
		};

		let action_kind = match event.kind {
			BridgeContractEvent::Initiated(detail) => {
				SwapActionType::MintLockCounterPart(intiator_address, counter_part_address)
			}
			BridgeContractEvent::Locked => {
				//transition event. The counterpart has been locked
				state.state = SwapStateType::Locked;
				SwapActionType::SwapLocked
			}
			BridgeContractEvent::InitialtorCompleted(secret) => {
				//transition event. Alice reveal the secret
				state.state = SwapStateType::KnowSecret;
				SwapActionType::ReleaseBurnInitiator
			}
			BridgeContractEvent::CounterPartCompleted => {
				//No transition, replay the release.
				SwapActionType::WaitThenReleaseBurnInitiator
			}
			BridgeContractEvent::Refunded => {
				//transition event. Swap is done
				state.state = SwapStateType::Done;
				SwapActionType::SwapDone
			}
			BridgeContractEvent::TimeoutEvent => {
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

	fn validate_state<A>(&mut self, event: &TransferEvent<A>) -> Result<(), InvalidEventError> {
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

//mod swapstate;

// use bridge_shared::{
// 	blockchain_service::AbstractBlockchainService,
// 	bridge_service::{BridgeService, BridgeServiceConfig},
// };
// use ethereum_bridge::{
// 	client::{Config as EthConfig, EthClient},
// 	event_monitoring::{EthCounterpartyMonitoring, EthInitiatorMonitoring},
// 	types::{EthAddress, EthHash},
// 	utils::TestRng,
// 	EthereumChain,
// };
// use movement_bridge::{
// 	client::{Config as MovementConfig, MovementClient},
// 	event_monitoring::{MovementCounterpartyMonitoring, MovementInitiatorMonitoring},
// 	utils::{MovementAddress, MovementHash},
// 	MovementChain,
// };
// use rand::SeedableRng;

// pub type EthereumService = AbstractBlockchainService<
// 	EthClient,
// 	EthInitiatorMonitoring<EthAddress, EthHash>,
// 	EthClient,
// 	EthCounterpartyMonitoring<EthAddress, EthHash>,
// 	EthAddress,
// 	EthHash,
// >;

// pub type MovementService = AbstractBlockchainService<
// 	MovementClient,
// 	MovementInitiatorMonitoring<MovementAddress, MovementHash>,
// 	MovementClient,
// 	MovementCounterpartyMonitoring<MovementAddress, MovementHash>,
// 	MovementAddress,
// 	MovementHash,
// >;

// pub struct SetupBridgeService(
// 	pub BridgeService<EthereumService, MovementService>,
// 	pub EthClient,
// 	pub MovementClient,
// 	pub EthereumChain,
// 	pub MovementChain,
// );

// pub async fn setup_bridge_service(bridge_config: BridgeServiceConfig) -> SetupBridgeService {
// 	let mut rng = TestRng::from_seed([0u8; 32]);
// 	let mut ethereum_service = EthereumChain::new("Ethereum".to_string(), "localhost:8545").await;
// 	let mut movement_service = MovementChain::new();

// 	//@TODO: use json config instead of build_for_test
// 	let config = EthConfig::build_for_test();

// 	let eth_client = EthClient::new(config).await.expect("Faile to creaet EthClient");
// 	let temp_rpc_url = "http://localhost:8545";
// 	let eth_initiator_monitoring = EthInitiatorMonitoring::build(temp_rpc_url.clone())
// 		.await
// 		.expect("Failed to create EthInitiatorMonitoring");
// 	let eth_conterparty_monitoring = EthCounterpartyMonitoring::build(temp_rpc_url)
// 		.await
// 		.expect("Failed to create EthCounterpartyMonitoring");

// 	let movement_counterparty_monitoring = MovementCounterpartyMonitoring::build("localhost:8080")
// 		.await
// 		.expect("Failed to create MovementCounterpartyMonitoring");
// 	let movement_initiator_monitoring = MovementInitiatorMonitoring::build("localhost:8080")
// 		.await
// 		.expect("Failed to create MovementInitiatorMonitoring");

// 	//@TODO: use json config instead of build_for_test
// 	let config = MovementConfig::build_for_test();

// 	let ethereum_chain = EthereumService {
// 		initiator_contract: eth_client.clone(),
// 		initiator_monitoring: eth_initiator_monitoring,
// 		counterparty_contract: eth_client.clone(),
// 		counterparty_monitoring: eth_conterparty_monitoring,
// 		_phantom: Default::default(),
// 	};

// 	let movement_client =
// 		MovementClient::new(config).await.expect("Failed to create MovementClient");

// 	let movement_chain = MovementService {
// 		initiator_contract: movement_client.clone(),
// 		initiator_monitoring: movement_initiator_monitoring,
// 		counterparty_contract: movement_client.clone(),
// 		counterparty_monitoring: movement_counterparty_monitoring,
// 		_phantom: Default::default(),
// 	};

// 	// EthereumChain must be BlockchainService
// 	let bridge_service = BridgeService::new(ethereum_chain, movement_chain, bridge_config);

// 	SetupBridgeService(
// 		bridge_service,
// 		eth_client,
// 		movement_client,
// 		ethereum_service,
// 		movement_service,
// 	)
// }
