use maptos_rest::{create_routes, MaptosConfig};
use poem::{listener::TcpListener, middleware::Tracing, EndpointExt, Server};

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
	let config = MaptosConfig::try_from_env().expect("Failed to parse config");

	let maptos_rest = create_routes().with(Tracing);

	Server::new(TcpListener::bind(&config.url)).run(maptos_rest).await
}

#[cfg(test)]
mod tests {
	use super::*;
	use poem::test::TestClient;

	#[tokio::test]
	async fn test_health() {
		let app = create_routes().with(Tracing);
		let cli = TestClient::new(app);

		// send request
		let resp = cli.get("/health").send().await;
		// check the status code
		resp.assert_status_is_ok();
		// check the body string
		resp.assert_text("healthy").await;
	}

	#[tokio::test]
	async fn test_state_root_hash() {
		let app = create_routes().with(Tracing);
		let cli = TestClient::new(app);

		// send request
		let resp = cli.get("/state_root_hash").send().await;
		// check the status code
		resp.assert_status_is_ok();
		// check the body string
		resp.assert_text("the_state_root_hash").await;
	}
}
