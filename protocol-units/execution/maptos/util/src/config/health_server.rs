use super::common::{default_health_server_hostname, default_health_server_port};
use anyhow::Error;
use poem::listener::TcpListener;
use poem::{get, handler, IntoResponse, Response, Route, Server};
use serde::{Deserialize, Serialize};

// An additional health server to be used by the indexer(or any other service).
// Do not use this with node since it exposes various endpoints to verify the health of the node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
	#[serde(default = "default_health_server_hostname")]
	pub hostname: String,
	#[serde(default = "default_health_server_port")]
	pub port: u16,
}

impl Default for Config {
	fn default() -> Self {
		Self { hostname: default_health_server_hostname(), port: default_health_server_port() }
	}
}

impl Config {
	pub async fn run(self) -> Result<(), anyhow::Error> {
		let url = format!("{}:{}", self.hostname, self.port);
		run_service(url).await
	}
}

pub async fn run_service(url: String) -> Result<(), Error> {
	let route = Route::new().at("/health", get(health));
	tracing::info!("Start health check access on :{url} .");
	Server::new(TcpListener::bind(url)).run(route).await.map_err(Into::into)
}

#[handler]
async fn health() -> Response {
	"{\"OK\": \"healthy\"}".into_response()
}
