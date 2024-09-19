use anyhow::Error;
use futures::prelude::*;
use poem::listener::TcpListener;
use poem::{get, handler, IntoResponse, Response, Route, Server};
use std::future::Future;

pub fn run_service(url: String) -> impl Future<Output = Result<(), Error>> + Send {
	let route = Route::new().at("/health", get(health));
	tracing::info!("Start health check access on {url} .");
	Server::new(TcpListener::bind(url)).run(route).map_err(Into::into)
}

#[handler]
async fn health() -> Response {
	"{\"OK\": \"healthy\"}".into_response()
}
