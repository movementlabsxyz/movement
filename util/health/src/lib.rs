use anyhow::Error;
use poem::listener::TcpListener;
use poem::{get, handler, IntoResponse, Response, Route, Server};

pub async fn run_service(hostname: String, port: u16) -> Result<(), Error> {
	let route = Route::new().at("/health", get(health));
	let url = format!("{}:{}", hostname, port);
	tracing::info!("Start health check access on :{url} .");
	Server::new(TcpListener::bind(url)).run(route).await.map_err(Into::into)
}

#[handler]
async fn health() -> Response {
	"{\"OK\": \"healthy\"}".into_response()
}
