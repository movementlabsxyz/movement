//use bridge_indexer_db::client::Client as IndexerClient;
use futures::stream::FuturesUnordered;
use tokio::select;
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use tokio_stream::StreamExt;

pub mod bridge_contracts;

pub async fn check_monitoring_health(
	chain: &str,
	healthcheck_source: mpsc::Sender<oneshot::Sender<bool>>,
	mut healthcheck_request_rx: mpsc::Receiver<oneshot::Sender<bool>>,
) -> Result<(), anyhow::Error> {
	let mut monitoring_health_check_interval =
		tokio::time::interval(tokio::time::Duration::from_secs(5));
	let mut health_status = true; // init health check with alive status.
	let mut health_check_result_futures = FuturesUnordered::new();

	loop {
		select! {
			// Health Check routine.
			// Manage REST HealthCheck request
			Some(oneshot_tx) = healthcheck_request_rx.recv() => {
				if let Err(err) = oneshot_tx.send(health_status){
					tracing::warn!("Heal check {chain} oneshot channel closed abnormally :{err:?}");
				}

			}
			// Verify that monitoring heath check still works.
			_ = monitoring_health_check_interval.tick() => {
				//Chain source monitoring health check.
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
						tracing::warn!("Health monitor:{chain} monitoring health check fail with an error:{err}",);
						health_status = false;
					},
					Err(err)=>{
						// Tokio execution fail. Process should exit.
						tracing::error!("Health monitor:{chain} , Error during health check tokio task execution exiting: {err}");
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
