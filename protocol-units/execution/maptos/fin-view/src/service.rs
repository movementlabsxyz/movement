use aptos_api::{
	runtime::{get_api_service, get_apis, Apis},
	Context,
};

use futures::prelude::*;
use poem::{http::Method, listener::TcpListener, middleware::Cors, EndpointExt, Route, Server};
use tracing::info;

use std::future::Future;
use std::sync::Arc;

#[derive(Clone)]
/// The API service for the finality view.
pub struct Service {
	context: Arc<Context>,
	listen_url: String,
}

impl Service {
	pub(crate) fn new(context: Arc<Context>, listen_url: String) -> Self {
		Service { context, listen_url }
	}

	pub fn get_apis(&self) -> Apis {
		get_apis(self.context.clone())
	}

	pub fn run(&self) -> impl Future<Output = Result<(), anyhow::Error>> + Send {
		info!("Starting maptos-fin-view services at: {:?}", self.listen_url);

		let api_service =
			get_api_service(self.context.clone()).server(format!("http://{:?}", self.listen_url));

		let ui = api_service.swagger_ui();

		let cors = Cors::new()
			.allow_methods(vec![Method::GET, Method::POST])
			.allow_credentials(true);
		let app = Route::new().nest("/v1", api_service).nest("/spec", ui).with(cors);

		Server::new(TcpListener::bind(self.listen_url.clone()))
			.run(app)
			.map_err(|e| anyhow::anyhow!("Server error: {:?}", e))
	}
}
