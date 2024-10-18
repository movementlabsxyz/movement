use anyhow::Error;
use aptos_api::Context;
use bridge_config::common::movement::MovementConfig;
use futures::prelude::*;
use poem::{
	get, handler, listener::TcpListener, middleware::Tracing, EndpointExt, IntoResponse, Response,
	Route, Server,
};
use std::env;
use std::future::Future;
use std::sync::Arc;
use tracing::info;

#[derive(Debug)]
pub struct BridgeRest {
	pub url: String,
	pub context: Option<Arc<Context>>,
}

impl BridgeRest {
	pub const BRIDGE_REST_ENV_VAR: &'static str = "BRIDGE_REST_URL";

	pub fn new(conf: MovementConfig) -> Result<Self, anyhow::Error> {
		let url = format!("http://{}:{}", conf.rest_hostname, conf.rest_port);

		Ok(Self { url, context: None })
	}

	pub fn set_context(&mut self, context: Arc<Context>) {
		self.context = Some(context);
	}

	pub fn run_service(&self) -> impl Future<Output = Result<(), Error>> + Send {
		info!("Starting Movement REST service at {}", self.url);
		let movement_rest = self.create_routes();
		Server::new(TcpListener::bind(self.url.clone()))
			.run(movement_rest)
			.map_err(Into::into)
	}

	pub fn create_routes(&self) -> impl EndpointExt {
		Route::new().at("/health", get(health)).with(Tracing)
	}
}

#[handler]
async fn health() -> Response {
	"OK".into_response()
}
