use crate::actions::process_action;
use crate::actions::TransferAction;
use crate::actions::TransferActionType;
use crate::chains::bridge_contracts::BridgeContract;
use crate::chains::bridge_contracts::BridgeContractError;
use crate::chains::bridge_contracts::BridgeContractEvent;
use crate::chains::bridge_contracts::BridgeContractMonitoring;
use crate::events::InvalidEventError;
use crate::events::TransferEvent;
use crate::states::TransferState;
use crate::types::BridgeTransferId;
use crate::types::ChainId;
use futures::stream::FuturesUnordered;
use std::collections::HashMap;
use tokio::select;
use tokio_stream::StreamExt;

mod actions;
pub mod chains;
mod events;
mod states;
pub mod types;

pub async fn run_bridge<
	A1: Send + From<Vec<u8>> + std::clone::Clone + 'static,
	A2: Send + From<Vec<u8>> + std::clone::Clone + 'static,
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

	let mut client_exec_result_futures = FuturesUnordered::new();

	loop {
		select! {
			// Wait on chain one events.
			Some(one_event_res) = one_stream.next() =>{
				match one_event_res {
					Ok(one_event) => {
						let event : TransferEvent<A1> = (one_event, ChainId::ONE).into();
						match state_runtime.process_event(event) {
							Ok(action) => {
								//Execute action
								let fut = process_action(action, one_client.clone());
								if let Some(fut) = fut {
									let jh = tokio::spawn(fut);
									client_exec_result_futures.push(jh);
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
						match state_runtime.process_event(event) {
							Ok(action) => {
								//Execute action
								let fut = process_action(action, two_client.clone());
								if let Some(fut) = fut {
									let jh = tokio::spawn(fut);
									client_exec_result_futures.push(jh);
								}
							},
							Err(err) => tracing::warn!("Received an invalid event: {err}"),
						}
					}
					Err(err) => tracing::error!("Chain two event stream return an error:{err}"),
				}
			}
			// Wait on client tx execution result.
			Some(jh) = client_exec_result_futures.next() => {
				match jh {
					//Client execution ok.
					Ok(Ok(_)) => (),
					Ok(Err(err)) => {
						// Manage Tx execution error
						state_runtime.process_client_exec_error(err);
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

	pub fn process_event<A: Into<Vec<u8>> + std::clone::Clone, B: From<Vec<u8>>>(
		&mut self,
		event: TransferEvent<A>,
	) -> Result<TransferAction<B>, InvalidEventError> {
		self.validate_state(&event)?;
		let event_transfer_id = event.contract_event.bridge_transfer_id();
		let state_opt = self.swap_state_map.remove(&event_transfer_id);
		//create swap state if need
		let mut state = if let BridgeContractEvent::Initiated(detail) = event.contract_event {
			let (state, action) =
				TransferState::transition_from_initiated(event.chain, event_transfer_id, detail);
			self.swap_state_map.insert(state.transfer_id, state);
			return Ok(action);
		} else {
			//tested before state can be unwrap
			state_opt.unwrap()
		};

		let action_kind = match event.contract_event {
			BridgeContractEvent::Initiated(_) => unreachable!(),
			BridgeContractEvent::Locked(detail) => {
				let (new_state, action_kind) =
					state.transition_from_locked_done(event_transfer_id, detail);
				state = new_state;
				action_kind
			}
			BridgeContractEvent::CounterPartCompleted(_, preimage) => {
				let (new_state, action_kind) = state
					.transition_from_counterpart_completed::<A, B>(event_transfer_id, preimage);
				state = new_state;
				action_kind
			}
			BridgeContractEvent::InitialtorCompleted(_) => {
				todo!()
			}
			BridgeContractEvent::Cancelled(_) => {
				todo!()
			}
			BridgeContractEvent::Refunded(_) => {
				todo!()
			}
		};

		let action = TransferAction {
			init_chain: state.init_chain,
			transfer_id: state.transfer_id,
			kind: action_kind,
		};

		self.swap_state_map.insert(state.transfer_id, state);

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

	fn process_client_exec_error(&mut self, error: BridgeContractError) {
		tracing::warn!("Client execution error:{error}");
		todo!();
	}
}
