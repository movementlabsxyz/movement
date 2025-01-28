use anyhow::Error;
use bridge_config::common::movement::MovementConfig;
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

struct RestContext {
	l1_request_tx: mpsc::Sender<oneshot::Sender<bool>>,
	l2_request_tx: mpsc::Sender<oneshot::Sender<bool>>,
}

pub struct BridgeRest {
	pub url: String,
	context: Arc<RestContext>,
}

impl BridgeRest {
	pub const BRIDGE_REST_ENV_VAR: &'static str = "BRIDGE_REST_URL";

	pub fn new(
		rest_listener_url: String,
		l1_request_tx: mpsc::Sender<oneshot::Sender<bool>>,
		l2_request_tx: mpsc::Sender<oneshot::Sender<bool>>,
	) -> Result<Self, anyhow::Error> {
		//		let url = format!("{}:{}", conf.rest_listener_hostname, conf.rest_port);

		let context = RestContext { l1_request_tx, l2_request_tx };
		Ok(Self { url: rest_listener_url, context: Arc::new(context) })
	}

	pub fn run_service(&self) -> impl Future<Output = Result<(), Error>> + Send {
		info!("Starting Movement REST service at {}", self.url);
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
async fn health(context: Data<&Arc<RestContext>>) -> Result<Response, anyhow::Error> {
	let (l1_tx, l1_rx) = oneshot::channel();
	context.l1_request_tx.send(l1_tx).await?;
	let (l2_tx, l2_rx) = oneshot::channel();
	context.l2_request_tx.send(l2_tx).await?;
	let l1_resp = tokio::time::timeout(std::time::Duration::from_secs(2), l1_rx).await??;
	let l2_resp = tokio::time::timeout(std::time::Duration::from_secs(2), l2_rx).await??;
	let res = if l1_resp && l2_resp { "OK".to_string() } else { format!("NOK") };
	Ok(res.into_response())
}
