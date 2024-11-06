use crate::actions::process_action;
use bridge_indexer_db::client::Client;
use bridge_util::{
	actions::{ActionExecError, TransferAction, TransferActionType},
	chains::bridge_contracts::{BridgeContract, BridgeContractEvent, BridgeContractMonitoring},
	events::{InvalidEventError, TransferEvent},
	states::{TransferState, TransferStateType},
	types::{BridgeTransferId, ChainId},
};
use futures::stream::FuturesUnordered;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use tokio::{select, sync::Mutex};
use tokio_stream::StreamExt;

pub use bridge_util::types;

mod actions;
pub mod chains;
pub mod grpc;
pub mod rest;

pub async fn run_bridge<
	A1: Send + TryFrom<Vec<u8>> + std::clone::Clone + 'static + std::fmt::Debug,
	A2: Send + TryFrom<Vec<u8>> + std::clone::Clone + 'static + std::fmt::Debug,
>(
	one_client: impl BridgeContract<A1> + 'static,
	mut one_stream: impl BridgeContractMonitoring<Address = A1>,
	two_client: impl BridgeContract<A2> + 'static,
	mut two_stream: impl BridgeContractMonitoring<Address = A2>,
	mut healthcheck_request_rx: mpsc::Receiver<oneshot::Sender<String>>,
) -> Result<(), anyhow::Error>
where
	Vec<u8>: From<A1>,
	Vec<u8>: From<A2>,
{
	let mut state_runtime = Runtime::new(None);

	let mut client_exec_result_futures_one = FuturesUnordered::new();
	let mut client_exec_result_futures_two = FuturesUnordered::new();

	//only one client can use at a time.
	let one_client_lock = Arc::new(Mutex::new(()));
	let two_client_lock = Arc::new(Mutex::new(()));

	let mut tranfer_log_interval = tokio::time::interval(tokio::time::Duration::from_secs(60));

	loop {
		select! {
			//Manage HealthCheck request
			Some(oneshot_tx) = healthcheck_request_rx.recv() => {
				if let Err(err) = oneshot_tx.send("OK".to_string()){
					tracing::warn!("Heal check oneshot channel closed abnormally :{err:?}");
				}

			}
			// Log all current transfer
			_ = tranfer_log_interval.tick() => {
				//format logs
				let logs: Vec<_> = state_runtime.iter_state().map(|state| state.to_string()).collect();
				tokio::spawn(async move {
					tracing::info!("Bridge current transfer processing:{:#?}", logs);
				});
			}
			// Wait on chain one events.
			Some(one_event_res) = one_stream.next() =>{
				match one_event_res {
					Ok(one_event) => {
						let event : TransferEvent<A1> = (one_event, ChainId::ONE).into();
						tracing::info!("Receive event from chain ONE:{} ", event.contract_event);
						match state_runtime.process_event(event) {
							Ok(action) => {
								//Execute action
								match action.chain {
									ChainId::ONE => {
										let fut = process_action(action, one_client.clone());
										if let Some(fut) = fut {
											let jh = tokio::spawn({
												let client_lock_clone = one_client_lock.clone();
												async move {
													let _lock = client_lock_clone.lock().await;
													fut.await
												}
											});
											client_exec_result_futures_one.push(jh);
										}

									},
									ChainId::TWO => {
										let fut = process_action(action, two_client.clone());
										if let Some(fut) = fut {
											let jh = tokio::spawn({
												let client_lock_clone = two_client_lock.clone();
												async move {
													let _lock = client_lock_clone.lock().await;
													fut.await
												}
											});
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
	indexer_db_client: Option<Client>,
}

impl Runtime {
	pub fn new(indexer_db_client: Option<Client>) -> Self {
		Runtime { swap_state_map: HashMap::new(), indexer_db_client }
	}

	pub fn iter_state(&self) -> impl Iterator<Item = &TransferState> {
		self.swap_state_map.values()
	}

	fn index_event<A>(&mut self, event: TransferEvent<A>) -> Result<(), InvalidEventError>
	where
		A: Into<Vec<u8>> + std::clone::Clone,
	{
		match self.indexer_db_client {
			Some(ref mut client) => {
				client.insert_bridge_contract_event(event.contract_event.clone()).map_err(
					|_| {
						tracing::warn!("Fail to index event");
						InvalidEventError::BadEvent
					},
				)?;
				Ok(())
			}
			None => {
				tracing::warn!("No indexer db client found. Event not indexed");
				Ok(())
			}
		}
	}

	pub fn index_transfer_action(
		&mut self,
		action: TransferAction,
	) -> Result<(), InvalidEventError> {
		match self.indexer_db_client {
			Some(ref mut client) => {
				client.insert_transfer_action(action.kind).map_err(|_| {
					tracing::warn!("Fail to index action");
					InvalidEventError::BadEvent
				})?;
				Ok(())
			}
			None => {
				tracing::warn!("No indexer db client found. Action not indexed");
				Ok(())
			}
		}
	}

	pub fn process_event<A>(
		&mut self,
		event: TransferEvent<A>,
	) -> Result<TransferAction, InvalidEventError>
	where
		A: Into<Vec<u8>> + std::clone::Clone,
	{
		self.validate_state(&event)?;
		let indexer_event = event.clone();
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
			//tested before in validate_state() state can be unwrap
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

		//index event
		self.index_event(indexer_event)?;

		let action =
			TransferAction { chain: chain_id, transfer_id: state.transfer_id, kind: action_kind };

		// index action
		// todo: really this should come after process_action completion, but the current use of process_action is hacky
		self.index_transfer_action(action.clone())?;

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
