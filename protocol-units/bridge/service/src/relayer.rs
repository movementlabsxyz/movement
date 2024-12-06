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
use tokio::sync::oneshot;
use tokio::{select, sync::Mutex};
use tokio_stream::StreamExt;

pub async fn run_relayer_one_direction<
	A1: Send + TryFrom<Vec<u8>> + std::clone::Clone + 'static + std::fmt::Debug,
	A2: Send + TryFrom<Vec<u8>> + std::clone::Clone + 'static + std::fmt::Debug,
>(
	direction: &str,
	mut stream_source: impl BridgeContractMonitoring<Address = A1>,
	healthcheck_source: mpsc::Sender<oneshot::Sender<bool>>,
	client_target: impl BridgeRelayerContract<A2> + 'static,
	mut stream_target: impl BridgeContractMonitoring<Address = A2>,
	mut healthcheck_request_rx: mpsc::Receiver<oneshot::Sender<bool>>,
	//	indexer_db_client: Option<IndexerClient>,
) -> Result<(), anyhow::Error>
where
	Vec<u8>: From<A1>,
	Vec<u8>: From<A2>,
{
	let mut state_runtime = Runtime::new(); //indexer_db_client

	let mut client_exec_result_futures = FuturesUnordered::new();

	//only one client can use at a time.
	let client_lock = Arc::new(Mutex::new(()));

	let mut tranfer_log_interval = tokio::time::interval(tokio::time::Duration::from_secs(60));
	let mut monitoring_health_check_interval =
		tokio::time::interval(tokio::time::Duration::from_secs(5));
	let mut health_status = true; // init health check with alive status.
	let mut health_check_result_futures = FuturesUnordered::new();

	loop {
		select! {
			// Wait on chain one events.
			Some(event_res) = stream_source.next() =>{
				match event_res {
					Ok(BridgeContractEvent::Initiated(detail)) => {
						let event : TransferEvent<A1> = BridgeContractEvent::Initiated(detail).into();
						tracing::info!("Relayer:{direction}, receive Initiated event :{} ", event.contract_event);
						process_event(event, &mut state_runtime, client_target.clone(), client_lock.clone(), &mut client_exec_result_futures);
					}
					Ok(_) => (), //do nothing for other event.
					Err(err) => tracing::error!("Relayer:{direction} event stream return an error:{err}"),
				}
			}
			Some(event_res) = stream_target.next() =>{
				match event_res {
					Ok(BridgeContractEvent::Completed(detail)) => {
						let event : TransferEvent<A2> = BridgeContractEvent::Completed(detail).into();
						tracing::info!("Relayer:{direction}, receive Completed event :{} ", event.contract_event);
						process_event(event, &mut state_runtime, client_target.clone(), client_lock.clone(), &mut client_exec_result_futures);
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
			// Health Check routine.
			// Manage REST HealthCheck request
			Some(oneshot_tx) = healthcheck_request_rx.recv() => {
				if let Err(err) = oneshot_tx.send(health_status){
					tracing::warn!("Heal check {direction} oneshot channel closed abnormally :{err:?}");
				}

			}
			// Verify that monitoring heath check still works.
			_ = monitoring_health_check_interval.tick() => {
				//Chain one monitoring health check.
				let jh = tokio::spawn({
					let healthcheck_tx = healthcheck_source.clone();
					async move {
						check_monitoring_loop_heath(healthcheck_tx).await
					}
				});
				health_check_result_futures.push(jh);
			}
			// Process health check result.
			Some(res) = health_check_result_futures.next() => {
				match res {
					//Client execution ok.
					Ok(Ok(status)) => health_status = status,
					Ok(Err(err)) => {
						tracing::warn!("Relayer:{direction}  monitoring health check fail with an error:{err}",);
						health_status = false;
					},
					Err(err)=>{
						// Tokio execution fail. Process should exit.
						tracing::error!("Relayer:{direction} , Error during health check tokio task execution exiting: {err}");
						return Err(err.into());
					}
				}
			}
			// Log all current transfer
			_ = tranfer_log_interval.tick() => {
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

fn process_event<
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
) where
	Vec<u8>: From<A>,
{
	match state_runtime.process_event(event) {
		Ok(action) => execute_action(
			action,
			state_runtime,
			client_target,
			tx_lock,
			client_exec_result_futures_one,
		),
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

async fn check_monitoring_loop_heath(
	healthcheck_tx: mpsc::Sender<oneshot::Sender<bool>>,
) -> Result<bool, String> {
	let (tx, rx) = oneshot::channel();
	healthcheck_tx
		.send(tx)
		.await
		.map_err(|err| format!("Health check send error: {}", err))?;
	let res = match tokio::time::timeout(tokio::time::Duration::from_secs(5), rx).await {
		Ok(Ok(res)) => res,
		Ok(Err(err)) => {
			tracing::warn!("Monitoring health check return an error:{err}");
			false
		}
		Err(_) => {
			tracing::warn!("Monitoring health check timeout. Monitoring is idle.");
			false
		}
	};
	Ok(res)
}
