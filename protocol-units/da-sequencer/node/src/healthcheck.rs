use anyhow::Error;
use futures::prelude::*;
use poem::{
	get, handler, listener::TcpListener, middleware::Tracing, web::Data, EndpointExt, IntoResponse,
	Response, Route, Server,
};
use std::future::Future;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use tracing::info;

pub const DEFAULT_REST_LISTENER_HOSTNAME: &str = "0.0.0.0";

struct HealthCheckContext {
	check_request_tx: mpsc::Sender<oneshot::Sender<bool>>,
}

pub struct HealthCheckRest {
	pub url: String,
	context: Arc<HealthCheckContext>,
}

impl HealthCheckRest {
	pub fn new(
		rest_listener_url: String,
		check_request_tx: mpsc::Sender<oneshot::Sender<bool>>,
	) -> Result<Self, anyhow::Error> {
		let context = HealthCheckContext { check_request_tx };
		Ok(Self { url: rest_listener_url, context: Arc::new(context) })
	}

	pub fn run_service(&self) -> impl Future<Output = Result<(), Error>> + Send {
		info!("Starting da-sequencer health check service at {}", self.url);
		let movement_rest = self.create_routes();
		Server::new(TcpListener::bind(self.url.clone()))
			.run(movement_rest)
			.map_err(Into::into)
	}

	pub fn create_routes(&self) -> impl EndpointExt {
		Route::new().at("/health", get(health)).with(Tracing).data(self.context.clone())
	}
}

#[handler]
async fn health(context: Data<&Arc<HealthCheckContext>>) -> Result<Response, anyhow::Error> {
	let (check_tx, check_rx) = oneshot::channel();
	context.check_request_tx.send(check_tx).await?;
	let check_resp = tokio::time::timeout(std::time::Duration::from_secs(2), check_rx).await??;
	let res = if check_resp { "OK".to_string() } else { format!("NOK") };
	Ok(res.into_response())
}
