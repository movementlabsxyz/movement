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
	request_tx: mpsc::Sender<oneshot::Sender<String>>,
}

pub struct BridgeRest {
	pub url: String,
	context: Arc<RestContext>,
}

impl BridgeRest {
	pub const BRIDGE_REST_ENV_VAR: &'static str = "BRIDGE_REST_URL";

	pub fn new(
		conf: &MovementConfig,
		request_tx: mpsc::Sender<oneshot::Sender<String>>,
	) -> Result<Self, anyhow::Error> {
		let url = format!("{}:{}", conf.rest_listener_hostname, conf.rest_port);

		let context = RestContext { request_tx };
		Ok(Self { url, context: Arc::new(context) })
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
	let (tx, rx) = oneshot::channel();
	tokio::time::timeout(std::time::Duration::from_secs(2), context.request_tx.send(tx)).await??;
	let resp = rx.await?;
	Ok(resp.into_response())
}
