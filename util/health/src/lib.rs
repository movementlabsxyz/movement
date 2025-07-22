use poem::listener::TcpListener;
use poem::{get, handler, IntoResponse, Response, Route, Server};
use anyhow::Context;

/// Run a health server on the given hostname and port.
/// It's considered fatal if the health server fails.
pub async fn run_service(hostname: String, port: u16) -> anyhow::Result<()> {
	let route = Route::new().at("/health", get(health));
	let url = format!("{}:{}", hostname, port);
	tracing::info!("Start health check access on :{url} .");
    Server::new(TcpListener::bind(url)).run(route).await.context("Failed to start health server")
}

#[handler]
async fn health() -> Response {
	"{\"OK\": \"healthy\"}".into_response()
}
