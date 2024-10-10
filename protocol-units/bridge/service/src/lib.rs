use crate::actions::process_action;
use crate::actions::ActionExecError;
use crate::actions::TransferAction;
use crate::actions::TransferActionType;
use crate::chains::bridge_contracts::BridgeContract;
use crate::chains::bridge_contracts::BridgeContractEvent;
use crate::chains::bridge_contracts::BridgeContractMonitoring;
use crate::events::InvalidEventError;
use crate::events::TransferEvent;
use crate::states::TransferState;
use crate::states::TransferStateType;
use crate::types::BridgeTransferId;
use crate::types::ChainId;
use futures::stream::FuturesUnordered;
use std::collections::HashMap;
use tokio::select;
use tokio::task::JoinError;
use tokio::task::JoinHandle;
use tokio_stream::StreamExt;

mod actions;
pub mod chains;
mod events;
mod states;
pub mod types;

pub async fn run_bridge<
	A1: Send + From<Vec<u8>> + std::clone::Clone + 'static + std::fmt::Debug,
	A2: Send + From<Vec<u8>> + std::clone::Clone + 'static + std::fmt::Debug,
>(
	one_client: impl BridgeContract<A1> + 'static,
	mut one_stream: impl BridgeContractMonitoring<Address = A1>,
	two_client: impl BridgeContract<A2> + 'static,
	mut two_stream: impl BridgeContractMonitoring<Address = A2>,
) -> Result<(), anyhow::Error>
where
	Vec<u8>: From<A1>,
	Vec<u8>: From<A2>,
{
	let mut state_runtime = Runtime::new();

	let mut client_exec_result_futures_one = FuturesUnordered::new();
	let mut client_exec_result_futures_two = FuturesUnordered::new();

	// let mut action_to_exec_futures_one = FuturesUnordered::new();
	// let mut action_to_exec_futures_two = FuturesUnordered::new();

	loop {
		select! {
			// Wait on chain one events.
			Some(one_event_res) = one_stream.next() =>{
				match one_event_res {
					Ok(one_event) => {
						let event : TransferEvent<A1> = (one_event, ChainId::ONE).into();
						tracing::info!("Receive event from chain ONE:{}", event.contract_event.bridge_transfer_id());
						match state_runtime.process_event(event) {
							Ok(action) => {
								//Execute action
								match action.chain {
									ChainId::ONE => {
										let fut = process_action(action, one_client.clone());
										if let Some(fut) = fut {
											let jh = tokio::spawn(fut);
											client_exec_result_futures_one.push(jh);
										}

									},
									ChainId::TWO => {
										let fut = process_action(action, two_client.clone());
										if let Some(fut) = fut {
											let jh = tokio::spawn(fut);
											client_exec_result_futures_two.push(jh);
										}
									}
								}
							},
							Err(err) => tracing::warn!("Received an invalid event: {err}"),
						}
					}
					Err(err) => tracing::error!("Chain one event stream return an error:{err}"),
				}
			}
			// Wait on chain two events.
			Some(two_event_res) = two_stream.next() =>{
				match two_event_res {
					Ok(two_event) => {
						let event : TransferEvent<A2> = (two_event, ChainId::TWO).into();
						tracing::info!("Receive event from chain TWO id:{}", event.contract_event.bridge_transfer_id());
						match state_runtime.process_event(event) {
							Ok(action) => {
								//Execute action
								match action.chain {
									ChainId::ONE => {
										let fut = process_action(action, one_client.clone());
										if let Some(fut) = fut {
											let jh = tokio::spawn(fut);
											client_exec_result_futures_one.push(jh);
										}

									},
									ChainId::TWO => {
										let fut = process_action(action, two_client.clone());
										if let Some(fut) = fut {
											let jh = tokio::spawn(fut);
											client_exec_result_futures_two.push(jh);
										}
									}
								}
							},
							Err(err) => tracing::warn!("Received an invalid event: {err}"),
						}
					}
					Err(err) => tracing::error!("Chain two event stream return an error:{err}"),
				}
			}
			// Wait on client tx execution result.
			Some(res) = client_exec_result_futures_one.next() => {
				match res {
					//Client execution ok.
					Ok(Ok(_)) => (),
					Ok(Err(err)) => {
						// Manage Tx execution error
						let action = state_runtime.process_action_exec_error(err);
						// TODO execute action the same way as normal event.
						// TODO refactor to avopid code duplication.
					}
					Err(err)=>{
						// Tokio execution fail. Process should exit.
						tracing::error!("Error during client tokio tasj execution exiting: {err}");
						return Err(err.into());
					}
				}
			}
			Some(res) = client_exec_result_futures_two.next() => {
				match res {
					//Client execution ok.
					Ok(Ok(_)) => (),
					Ok(Err(err)) => {
						// Manage Tx execution error
						let action = state_runtime.process_action_exec_error(err);
						// TODO execute action the same way as normal event.
						// TODO refactor to avopid code duplication.
					}
					Err(err)=>{
						// Tokio execution fail. Process should exit.
						tracing::error!("Error during client tokio tasj execution exiting: {err}");
						return Err(err.into());
					}
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

	pub fn process_event<A>(
		&mut self,
		event: TransferEvent<A>,
	) -> Result<TransferAction, InvalidEventError>
	where
		A: Into<Vec<u8>> + std::clone::Clone,
	{
		self.validate_state(&event)?;
		let event_transfer_id = event.contract_event.bridge_transfer_id();
		let state_opt = self.swap_state_map.remove(&event_transfer_id);
		//create swap state if need
		let mut state = if let BridgeContractEvent::Initiated(detail) = event.contract_event {
			let (state, mut action) =
				TransferState::transition_from_initiated(event.chain, event_transfer_id, detail);
			action.chain = state.init_chain.other();
			self.swap_state_map.insert(state.transfer_id, state);
			return Ok(action);
		} else {
			//tested before state can be unwrap
			state_opt.unwrap()
		};

		let (action_kind, chain_id) = match event.contract_event {
			BridgeContractEvent::Initiated(_) => unreachable!(),
			BridgeContractEvent::Locked(detail) => {
				let (new_state, action_kind) =
					state.transition_from_locked_done(event_transfer_id, detail);
				state = new_state;
				(action_kind, state.init_chain)
			}
			BridgeContractEvent::CounterPartCompleted(_, preimage) => {
				let (new_state, action_kind) =
					state.transition_from_counterpart_completed(event_transfer_id, preimage);
				state = new_state;
				(action_kind, state.init_chain)
			}
			BridgeContractEvent::InitialtorCompleted(_) => {
				let (new_state, action_kind) =
					state.transition_from_initiator_completed(event_transfer_id);
				state = new_state;

				(action_kind, state.init_chain)
			}
			BridgeContractEvent::Cancelled(_) => {
				let (new_state, action_kind) = state.transition_from_cancelled(event_transfer_id);
				state = new_state;

				(action_kind, state.init_chain)
			}
			BridgeContractEvent::Refunded(_) => {
				let (new_state, action_kind) = state.transition_from_refunded(event_transfer_id);
				state = new_state;

				(action_kind, state.init_chain)
			}
		};

		let action =
			TransferAction { chain: chain_id, transfer_id: state.transfer_id, kind: action_kind };

		if state.state != TransferStateType::Done {
			self.swap_state_map.insert(state.transfer_id, state);
		}
		Ok(action)
	}

	fn validate_state<A>(&mut self, event: &TransferEvent<A>) -> Result<(), InvalidEventError> {
		let event_transfer_id = event.contract_event.bridge_transfer_id();
		let swap_state_opt = self.swap_state_map.get(&event_transfer_id);
		//validate the associated swap_state.
		swap_state_opt
			.as_ref()
			//if the state is present validate the event is compatible
			.map(|state| state.validate_event(&event))
			//if not validate the event is BridgeContractEvent::Initiated
			.or_else(|| {
				Some(
					(swap_state_opt.is_none() && event.contract_event.is_initiated_event())
						.then_some(())
						.ok_or(InvalidEventError::StateNotFound),
				)
			})
			.transpose()?;
		Ok(())
	}

	fn process_action_exec_error(&mut self, action_err: ActionExecError) -> Option<TransferAction> {
		// Manage Tx execution error
		let (action, err) = action_err.inner();
		tracing::warn!("Client execution error for action:{action:?} err:{err:?}");
		// retry 5 time an action in error then abort.
		match self.swap_state_map.get_mut(&action.transfer_id) {
			Some(state) => {
				state.retry_on_error += 1;
				if state.retry_on_error > 5 {
					// Depending on the action cancel transfer
					match action.kind {
						TransferActionType::LockBridgeTransfer { .. } => {
							//Lock fail. Refund initiator
							let (new_state_type, action_kind) = state.transition_to_refund();
							state.state = new_state_type;
							let action = TransferAction {
								chain: state.init_chain,
								transfer_id: state.transfer_id,
								kind: action_kind,
							};
							Some(action)
						}
						TransferActionType::WaitAndCompleteInitiator(..) => {
							todo!()
						}
						TransferActionType::RefundInitiator => None, //will wait automatic refund
						TransferActionType::TransferDone => None,
						TransferActionType::NoAction => None,
					}
				} else {
					//Rerun the action.
					Some(action)
				}
			}
			None => {
				tracing::warn!(
					"Receive an error for action but no state found for id:{:?}",
					action.transfer_id
				);
				None
			}
		}
	}
}
