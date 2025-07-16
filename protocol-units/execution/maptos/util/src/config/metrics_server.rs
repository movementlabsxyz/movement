use super::common::{default_metrics_server_hostname, default_metrics_server_port};
use poem::http::StatusCode;
use poem::{get, handler, listener::TcpListener, IntoResponse, Route, Server};
use prometheus::{gather, Encoder, TextEncoder};
use serde::{Deserialize, Serialize};
use tokio::task::JoinHandle;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MetricsConfig {
	#[serde(default = "default_metrics_server_hostname")]
	pub listen_hostname: String,
	#[serde(default = "default_metrics_server_port")]
	pub listen_port: u16,
}

impl Default for MetricsConfig {
	fn default() -> Self {
		Self {
			listen_hostname: default_metrics_server_hostname(),
			listen_port: default_metrics_server_port(),
		}
	}
}

impl MetricsConfig {
	pub async fn start_metrics_server(&self) -> Result<JoinHandle<()>, anyhow::Error> {
		let bind_address = format!("{}:{}", self.listen_hostname, self.listen_port);

		let metrics_route = Route::new().at("/metrics", get(metrics_handler));

		let server_handle = tokio::spawn(async move {
			let listener = TcpListener::bind(&bind_address);
			tracing::info!("Starting Prometheus metrics server on http://{}/metrics", bind_address);

			if let Err(e) = Server::new(listener).run(metrics_route).await {
				tracing::error!("Metrics server error: {}", e);
			}
		});

		Ok(server_handle)
	}
}

#[handler]
async fn metrics_handler() -> impl IntoResponse {
	let metrics = gather();
	let encoder = TextEncoder::new();
	let mut buffer = vec![];

	match encoder.encode(&metrics, &mut buffer) {
		Ok(_) => match String::from_utf8(buffer) {
			Ok(metrics_text) => poem::Response::builder()
				.status(StatusCode::OK)
				.header("content-type", "text/plain")
				.body(metrics_text),
			Err(_) => poem::Response::builder()
				.status(StatusCode::INTERNAL_SERVER_ERROR)
				.body("Error encoding metrics"),
		},
		Err(_) => poem::Response::builder()
			.status(StatusCode::INTERNAL_SERVER_ERROR)
			.body("Error gathering metrics"),
	}
}
