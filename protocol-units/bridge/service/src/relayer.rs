use crate::actions;
use crate::runtime::Runtime;
//use bridge_indexer_db::client::Client as IndexerClient;
use bridge_util::{
	actions::{ActionExecError, TransferAction},
	chains::bridge_contracts::{
		BridgeContractEvent, BridgeContractMonitoring, BridgeRelayerContract,
	},
	events::TransferEvent,
};
use futures::stream::FuturesUnordered;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::{select, sync::Mutex};
use tokio_stream::StreamExt;

pub async fn run_relayer_one_direction<
	SOURCE: Send + TryFrom<Vec<u8>> + std::clone::Clone + 'static + std::fmt::Debug,
	TARGET: Send + TryFrom<Vec<u8>> + std::clone::Clone + 'static + std::fmt::Debug,
>(
	direction: &str,
	mut stream_source: impl BridgeContractMonitoring<Address = SOURCE>,
	client_target: impl BridgeRelayerContract<TARGET> + 'static,
	mut stream_target: impl BridgeContractMonitoring<Address = TARGET>,
	action_sender: Option<mpsc::Sender<TransferAction>>,
) -> Result<(), anyhow::Error>
where
	Vec<u8>: From<SOURCE>,
	Vec<u8>: From<TARGET>,
{
	let mut state_runtime = Runtime::new(); //indexer_db_client

	let mut client_exec_result_futures = FuturesUnordered::new();

	//only one client can use at a time.
	let client_lock = Arc::new(Mutex::new(()));

	let mut transfer_log_interval = tokio::time::interval(tokio::time::Duration::from_secs(60));

	loop {
		select! {
			// Wait on chain one events.
			Some(event_res) = stream_source.next() =>{
				match event_res {
					Ok(BridgeContractEvent::Initiated(detail)) => {
						let event : TransferEvent<SOURCE> = BridgeContractEvent::Initiated(detail).into();
						tracing::info!("Relayer:{direction}, receive Initiated event :{} ", event.contract_event);
						process_event(event, &mut state_runtime, client_target.clone(), client_lock.clone(), &mut client_exec_result_futures, action_sender.as_ref().cloned()).await;
					}
					Ok(_) => (), //do nothing for other event.
					Err(err) => tracing::error!("Relayer:{direction} event stream return an error:{err}"),
				}
			}
			Some(event_res) = stream_target.next() =>{
				match event_res {
					Ok(BridgeContractEvent::Completed(detail)) => {
						let event : TransferEvent<TARGET> = BridgeContractEvent::Completed(detail).into();
						tracing::info!("Relayer:{direction}, receive Completed event :{} ", event.contract_event);
						process_event(event, &mut state_runtime, client_target.clone(), client_lock.clone(), &mut client_exec_result_futures, action_sender.as_ref().cloned()).await;
					}
					Ok(_) => (), //do nothing for other event.
					Err(err) => tracing::error!("Relayer:{direction} event stream return an error:{err}"),
				}
			}
			// Wait on client tx execution result.
			Some(res) = client_exec_result_futures.next() => {
				match res {
					//Client execution ok.
					Ok(Ok(_)) => (),
					Ok(Err(err)) => {
						// Manage Tx execution error
						if let Some(action) = state_runtime.process_action_exec_error(err) {
							execute_action(action, &mut state_runtime, client_target.clone(), client_lock.clone(), &mut client_exec_result_futures);
						}
					}
					Err(err)=>{
						// Tokio execution fail. Process should exit.
						tracing::error!("Relayer:{direction}, Error during client tokio task execution exiting: {err}");
						return Err(err.into());
					}
				}
			}
			// Log all current transfer
			_ = transfer_log_interval.tick() => {
				//format logs
				let logs: Vec<_> = state_runtime.iter_state().map(|state| state.to_string()).collect();
				tokio::spawn({
					let direction = direction.to_string();
					async move {
						tracing::info!("Relayer:{direction} current transfer processing:{:#?}", logs);
					}
				});
			}
		}
	}
}

async fn process_event<
	A: std::clone::Clone + std::fmt::Debug,
	TARGET: Send + TryFrom<Vec<u8>> + std::clone::Clone + 'static + std::fmt::Debug,
>(
	event: TransferEvent<A>,
	state_runtime: &mut Runtime,
	client_target: impl BridgeRelayerContract<TARGET> + 'static,
	tx_lock: Arc<Mutex<()>>,
	client_exec_result_futures_one: &mut FuturesUnordered<
		tokio::task::JoinHandle<Result<(), ActionExecError>>,
	>,
	action_sender: Option<mpsc::Sender<TransferAction>>,
) where
	Vec<u8>: From<A>,
{
	match state_runtime.process_event(event) {
		Ok(action) => {
			if let Some(sender) = action_sender {
				// Send in its own task so that the main processing task is never blocked if the send blocks.
				tokio::spawn({
					let action = action.clone();
					async move {
						if let Err(err) = sender.send(action).await {
							tracing::info!("Erreur when sending action to indexer sink: {err}",);
						}
					}
				});
			}
			execute_action(
				action,
				state_runtime,
				client_target,
				tx_lock,
				client_exec_result_futures_one,
			)
		}
		Err(err) => tracing::warn!("Received an invalid event: {err}"),
	}
}

fn execute_action<
	TARGET: Send + TryFrom<Vec<u8>> + std::clone::Clone + 'static + std::fmt::Debug,
>(
	action: TransferAction,
	state_runtime: &mut Runtime,
	client_target: impl BridgeRelayerContract<TARGET> + 'static,
	tx_lock: Arc<Mutex<()>>,
	client_exec_result_futures_one: &mut FuturesUnordered<
		tokio::task::JoinHandle<Result<(), ActionExecError>>,
	>,
) {
	let fut = actions::process_action(action, state_runtime, client_target);
	if let Some(fut) = fut {
		let jh = tokio::spawn({
			async move {
				let _lock = tx_lock.lock().await;
				fut.await
			}
		});
		client_exec_result_futures_one.push(jh);
	}
}
