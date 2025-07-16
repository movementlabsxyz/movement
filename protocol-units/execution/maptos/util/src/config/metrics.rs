use super::common::{
	default_ecosystem_metrics_listen_hostname, default_ecosystem_metrics_listen_port,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::task::JoinHandle;
use prometheus::{Encoder, TextEncoder, Registry, gather};
use poem::{listener::TcpListener, get, handler, Route, Server, EndpointExt, IntoResponse};
use poem::http::StatusCode;

// Metrics configuration that allows to add to existing services.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MetricsConfig {
	#[serde(default = "default_ecosystem_metrics_listen_hostname")]
	pub listen_hostname: String,
	#[serde(default = "default_ecosystem_metrics_listen_port")]
	pub listen_port: u16,
}

impl MetricsConfig {
	pub async fn start_metrics_server(&self) -> Result<JoinHandle<()>, anyhow::Error> {
		let bind_address = format!("{}:{}", self.listen_hostname, self.listen_port);
		
		// Set up the metrics endpoint route
		let metrics_route = Route::new()
			.at("/metrics", get(metrics_handler));
		
		// Start the metrics server
		let server_handle = tokio::spawn(async move {
			let listener = TcpListener::bind(&bind_address);
			
			tracing::info!("Starting Prometheus metrics server on http://{}/metrics", bind_address);
			
			if let Err(e) = Server::new(listener).run(metrics_route).await {
				tracing::error!("Metrics server error: {}", e);
			}
		});
		
		// Give the server a moment to start
		tokio::time::sleep(std::time::Duration::from_millis(100)).await;
		
		Ok(server_handle)
	}
	
	/// Get the metrics endpoint URL for this configuration
	pub fn metrics_endpoint(&self) -> String {
		format!("http://{}:{}/metrics", self.listen_hostname, self.listen_port)
	}
}

/// Handler for the /metrics endpoint that serves Prometheus-formatted metrics
#[handler]
async fn metrics_handler() -> impl IntoResponse {
	// Collect metrics from the global registry (includes all registered metrics)
	let global_metrics = gather();
	
	// Encode metrics in Prometheus text format
	let encoder = TextEncoder::new();
	let mut buffer = vec![];
	
	match encoder.encode(&global_metrics, &mut buffer) {
		Ok(_) => {
			// Convert to string and return with proper content type
			match String::from_utf8(buffer) {
				Ok(metrics_text) => {
					poem::Response::builder()
						.status(StatusCode::OK)
						.header("content-type", "text/plain; version=0.0.4; charset=utf-8")
						.body(metrics_text)
				}
				Err(e) => {
					tracing::error!("Failed to convert metrics to string: {}", e);
					poem::Response::builder()
						.status(StatusCode::INTERNAL_SERVER_ERROR)
						.body("Error encoding metrics")
				}
			}
		}
		Err(e) => {
			tracing::error!("Failed to encode metrics: {}", e);
			poem::Response::builder()
				.status(StatusCode::INTERNAL_SERVER_ERROR)
				.body("Error encoding metrics")
		}
	}
}
