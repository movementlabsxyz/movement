use crate::actions::process_action;
use bridge_indexer_db::client::Client as IndexerClient;
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

#[derive(Debug)]
struct HeathCheckStatus {
	chain_one: bool,
	chain_two: bool,
}

impl HeathCheckStatus {
	fn new() -> Self {
		HeathCheckStatus { chain_one: true, chain_two: true }
	}
	fn check(&self) -> bool {
		self.chain_one && self.chain_two
	}

	fn update_state(&mut self, chain: ChainId, state: bool) {
		match chain {
			ChainId::ONE => self.chain_one = state,
			ChainId::TWO => self.chain_two = state,
		}
	}
}

pub async fn run_bridge<
	A1: Send + TryFrom<Vec<u8>> + std::clone::Clone + 'static + std::fmt::Debug,
	A2: Send + TryFrom<Vec<u8>> + std::clone::Clone + 'static + std::fmt::Debug,
>(
	client_one: impl BridgeContract<A1> + 'static,
	mut stream_one: impl BridgeContractMonitoring<Address = A1>,
	client_two: impl BridgeContract<A2> + 'static,
	mut stream_two: impl BridgeContractMonitoring<Address = A2>,
	mut healthcheck_request_rx: mpsc::Receiver<oneshot::Sender<String>>,
	indexer_db_client: Option<IndexerClient>,
	healthcheck_tx_one: mpsc::Sender<oneshot::Sender<bool>>,
	healthcheck_tx_two: mpsc::Sender<oneshot::Sender<bool>>,
) -> Result<(), anyhow::Error>
where
	Vec<u8>: From<A1>,
	Vec<u8>: From<A2>,
{
	let mut state_runtime = Runtime::new(indexer_db_client);

	let mut client_exec_result_futures_one = FuturesUnordered::new();
	let mut client_exec_result_futures_two = FuturesUnordered::new();
	let mut health_check_result_futures = FuturesUnordered::new();

	//only one client can use at a time.
	let client_lock_one = Arc::new(Mutex::new(()));
	let client_lock_two = Arc::new(Mutex::new(()));

	let mut tranfer_log_interval = tokio::time::interval(tokio::time::Duration::from_secs(60));
	let mut monitoring_health_check_interval =
		tokio::time::interval(tokio::time::Duration::from_secs(5));

	let mut health_status = HeathCheckStatus::new();

	loop {
		select! {
			//Manage REST HealthCheck request
			Some(oneshot_tx) = healthcheck_request_rx.recv() => {
				let res = if health_status.check() {
					"OK".to_string()
				} else {
					format!("NOK : {health_status:?}")
				};
				if let Err(err) = oneshot_tx.send(res){
					tracing::warn!("Heal check oneshot channel closed abnormally :{err:?}");
				}

			}
			// verify that monitoring heath check still works.
			_ = monitoring_health_check_interval.tick() => {
				//Chain one monitoring health check.
				let jh = tokio::spawn({
					let healthcheck_tx = healthcheck_tx_one.clone();
					async move {
						(ChainId::ONE, check_monitoring_loop_heath(healthcheck_tx).await)
					}
				});
				health_check_result_futures.push(jh);
				//Chain two monitoring health check.
				let jh = tokio::spawn({
					let healthcheck_tx = healthcheck_tx_two.clone();
					async move {
						(ChainId::TWO, check_monitoring_loop_heath(healthcheck_tx).await)
					}
				});
				health_check_result_futures.push(jh);
			}
			// Process health check result.
			Some(res) = health_check_result_futures.next() => {
				match res {
					//Client execution ok.
					Ok((chain, Ok(status))) => health_status.update_state(chain, status),
					Ok((chain, Err(err))) => {
						tracing::warn!("Chain {chain} monitoring health check fail with an error:{err}",);
						health_status.update_state(chain, false);
					},
					Err(err)=>{
						// Tokio execution fail. Process should exit.
						tracing::error!("Error during health check tokio task execution exiting: {err}");
						return Err(err.into());
					}
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
			Some(event_res_one) = stream_one.next() =>{
				match event_res_one {
					Ok(event_one) => {
						let event : TransferEvent<A1> = (event_one, ChainId::ONE).into();
						tracing::info!("Receive event from chain ONE:{} ", event.contract_event);
						match state_runtime.process_event(event) {
							Ok(action) => {
								//Execute action
								match action.chain {
									ChainId::ONE => {
										let fut = process_action(action, client_one.clone());
										if let Some(fut) = fut {
											let jh = tokio::spawn({
												let client_lock_clone = client_lock_one.clone();
												async move {
													let _lock = client_lock_clone.lock().await;
													fut.await
												}
											});
											client_exec_result_futures_one.push(jh);
										}

									},
									ChainId::TWO => {
										let fut = process_action(action, client_two.clone());
										if let Some(fut) = fut {
											let jh = tokio::spawn({
												let client_lock_clone = client_lock_two.clone();
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
			Some(event_res_two) = stream_two.next() =>{
				match event_res_two {
					Ok(event_two) => {
						let event : TransferEvent<A2> = (event_two, ChainId::TWO).into();
						tracing::info!("Receive event from chain TWO :{}", event.contract_event);
						match state_runtime.process_event(event) {
							Ok(action) => {
								//Execute action
								match action.chain {
									ChainId::ONE => {
										let fut = process_action(action, client_one.clone());
										if let Some(fut) = fut {
											let jh = tokio::spawn(fut);
											client_exec_result_futures_one.push(jh);
										}

									},
									ChainId::TWO => {
										let fut = process_action(action, client_two.clone());
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
						tracing::error!("Error during client tokio task execution exiting: {err}");
						return Err(err.into());
					}
				}
			}
		}
	}
}

async fn check_monitoring_loop_heath(
	healthcheck_tx: mpsc::Sender<oneshot::Sender<bool>>,
) -> Result<bool, String> {
	let (tx, rx) = oneshot::channel();
	healthcheck_tx
		.send(tx)
		.await
		.map_err(|err| format!("Chain one Health check send error: {}", err))?;
	let res = match tokio::time::timeout(tokio::time::Duration::from_secs(5), rx).await {
		Ok(Ok(res)) => res,
		Ok(Err(err)) => {
			tracing::warn!("Chain one monitoring health check return an error:{err}");
			false
		}
		Err(_) => {
			tracing::warn!("Chain one monitoring health check timeout. Monitoring is idle.");
			false
		}
	};
	Ok(res)
}

struct Runtime {
	swap_state_map: HashMap<BridgeTransferId, TransferState>,
	indexer_db_client: Option<IndexerClient>,
}

impl Runtime {
	pub fn new(indexer_db_client: Option<IndexerClient>) -> Self {
		Runtime { swap_state_map: HashMap::new(), indexer_db_client }
	}

	pub fn iter_state(&self) -> impl Iterator<Item = &TransferState> {
		self.swap_state_map.values()
	}

	fn index_event<A>(&mut self, event: TransferEvent<A>) -> Result<(), InvalidEventError>
	where
		A: Into<Vec<u8>> + std::clone::Clone + std::fmt::Debug,
	{
		match self.indexer_db_client {
			Some(ref mut client) => {
				let event = event.contract_event;

				client.insert_bridge_contract_event(event.clone()).map_err(|err| {
					tracing::warn!("Fail to index event :{err}");
					InvalidEventError::IndexingFailed(err.to_string())
				})?;
				tracing::info!("index_event(success):{event:?}");
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
				let action = action.kind;
				client.insert_transfer_action(action.clone()).map_err(|err| {
					tracing::warn!("Fail to index action");
					InvalidEventError::BadEvent(err.to_string())
				})?;
				tracing::info!("index_transfer_action(success): {action:?}");
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
		A: Into<Vec<u8>> + std::clone::Clone + std::fmt::Debug,
	{
		tracing::info!("Event received: {:?}", event);
		self.validate_state(&event)?;
		let indexer_event = event.clone();
		self.index_event(indexer_event)?;
		let event_transfer_id = event.contract_event.bridge_transfer_id();
		let state_opt = self.swap_state_map.remove(&event_transfer_id);
		//create swap state if need
		let mut state = if let BridgeContractEvent::Initiated(detail) = event.contract_event {
			let (state, mut action) =
				TransferState::transition_from_initiated(event.chain, event_transfer_id, detail);
			action.chain = state.init_chain.other();
			self.swap_state_map.insert(state.transfer_id, state);
			self.index_transfer_action(action.clone())?;
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
			BridgeContractEvent::CounterPartyCompleted(_, preimage) => {
				let (new_state, action_kind) =
					state.transition_from_counterpart_completed(event_transfer_id, preimage);
				state = new_state;
				(action_kind, state.init_chain)
			}
			BridgeContractEvent::InitiatorCompleted(_) => {
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

		// index action
		// todo: really this should come after process_action completion, but the current use of process_action is hacky
		self.index_transfer_action(action.clone())?;

		if state.state != TransferStateType::Done {
			self.swap_state_map.insert(state.transfer_id, state);
		}
		Ok(action)
	}

	fn validate_state<A: std::fmt::Debug>(
		&mut self,
		event: &TransferEvent<A>,
	) -> Result<(), InvalidEventError> {
		let event_transfer_id = event.contract_event.bridge_transfer_id();
		tracing::info!("Validating event with transfer ID: {:?}", event_transfer_id);
		let swap_state_opt = self.swap_state_map.get(&event_transfer_id);

		// Log the current state if it exists in the swap state map
		if let Some(state) = swap_state_opt {
			tracing::info!(
				"Found existing state for transfer ID {:?}: {:?}",
				event_transfer_id,
				state.state
			);
		} else {
			tracing::info!("No existing state found for transfer ID {:?}", event_transfer_id);
		}
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
		tracing::warn!("Client execution error for action:{action} err:{err}");
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
