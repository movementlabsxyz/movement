use super::common::{
	default_ecosystem_health_check_listen_hostname, default_ecosystem_health_check_listen_port,
	default_maptos_indexer_grpc_listen_hostname, default_maptos_indexer_grpc_listen_port,
};
use poem::http::StatusCode;
use poem::listener::TcpListener;
use poem::web::Data;
use poem::EndpointExt;
use poem::{get, handler, Response, Route, Server};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::{channel, Receiver};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HealthCheckConfig {
	#[serde(default = "default_ecosystem_health_check_listen_hostname")]
	pub listen_hostname: String,
	#[serde(default = "default_ecosystem_health_check_listen_port")]
	pub listen_port: u16,
	#[serde(default = "default_maptos_indexer_grpc_listen_hostname")]
	pub grpc_hostname: String,
	#[serde(default = "default_maptos_indexer_grpc_listen_port")]
	pub grpc_port: u16,
}

#[handler]
async fn health_handler() -> Response {
	// Check gRPC indexer connection
	let grpc_url = format!("http://{}:{}", 
		default_maptos_indexer_grpc_listen_hostname(),
		default_maptos_indexer_grpc_listen_port()
	);
	
	match check_grpc_connection(&grpc_url).await {
		Ok(_) => Response::builder()
			.status(StatusCode::OK)
			.body("{\"status\": \"healthy\", \"grpc\": \"connected\"}"),
		Err(e) => Response::builder()
			.status(StatusCode::INTERNAL_SERVER_ERROR)
			.body(format!("{{\"status\": \"unhealthy\", \"grpc\": \"failed\", \"error\": \"{}\"}}", e))
	}
}

async fn check_grpc_connection(url: &str) -> Result<(), String> {
	let client = reqwest::Client::builder()
		.http2_prior_knowledge()
		.timeout(std::time::Duration::from_secs(5))
		.build()
		.map_err(|e| format!("Failed to create client: {}", e))?;
	
	let response = client
		.get(url)
		.header("Content-Type", "application/grpc")
		.send()
		.await
		.map_err(|e| format!("Connection failed: {}", e))?;
	
	if response.status().is_success() || response.status() == 405 {
		Ok(()) // 405 Method Not Allowed is normal for gRPC GET requests
	} else {
		Err(format!("gRPC server returned status: {}", response.status()))
	}
}

impl HealthCheckConfig {
	pub async fn start_health_check_server(&self) -> Result<(), anyhow::Error> {
        let app = Route::new().at("/health", get(health_handler));

        let server = Server::new(TcpListener::bind(format!("{}:{}", self.listen_hostname, self.listen_port)));
        server.run(app).await?;

        Ok(())
    }
}