use poem::http::StatusCode;
use poem::{get, handler, listener::TcpListener, IntoResponse, Route, Server};
use prometheus::{gather, Encoder, TextEncoder};
use tokio::task::JoinHandle;

/// Start a simple metrics server on the given hostname and port. This is for the usage other than the node.
pub async fn start_metrics_server(listen_hostname: String, listen_port: u16) -> Result<JoinHandle<()>, anyhow::Error> {
    let bind_address = format!("{}:{}", listen_hostname, listen_port);

    let metrics_route = Route::new().at("/metrics", get(metrics_handler));

    let server_handle = tokio::spawn(async move {
        let listener = TcpListener::bind(&bind_address);
        aptos_logger::info!("Starting Prometheus metrics server on http://{}/metrics", bind_address);

        if let Err(e) = Server::new(listener).run(metrics_route).await {
            aptos_logger::error!("Metrics server error: {}", e);
        }
    });

    Ok(server_handle)
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
