use anyhow::Result;
use bridge_config::Config;
use bridge_grpc::{
	bridge_server::BridgeServer, health_check_response::ServingStatus, health_server::HealthServer,
};
use bridge_indexer_db::client::Client;
use bridge_service::{
	chains::{
		ethereum::{client::EthClient, event_monitoring::EthMonitoring},
		movement::{
			client_framework::MovementClientFramework, event_monitoring::MovementMonitoring,
		},
	},
	grpc::HealthCheckService,
	rest::BridgeRest,
};
use godfig::{backend::config_file::ConfigFile, Godfig};
use std::net::SocketAddr;
use tonic::transport::Server;

#[tokio::main]
async fn main() -> Result<()> {
	use tracing_subscriber::EnvFilter;

	tracing_subscriber::fmt()
		.with_env_filter(
			EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
		)
		.init();

	tracing::info!("Start Bridge");

	// Define bridge config path
	let mut dot_movement = dot_movement::DotMovement::try_from_env()?;
	let pathbuff = bridge_config::get_config_path(&dot_movement);
	dot_movement.set_path(pathbuff);

	let config_file = dot_movement.try_get_or_create_config_file().await?;

	// Get a matching godfig object
	let godfig: Godfig<Config, ConfigFile> = Godfig::new(ConfigFile::new(config_file), vec![]);
	let bridge_config: Config = godfig.try_wait_for_ready().await?;

	tracing::info!("Bridge config loaded: {bridge_config:?}");

	let (eth_health_tx, eth_health_rx) = tokio::sync::mpsc::channel(10);
	let one_stream = EthMonitoring::build(&bridge_config.eth, eth_health_rx).await.unwrap();
	let one_client = EthClient::new(&bridge_config.eth).await.unwrap();
	let two_client = MovementClientFramework::new(&bridge_config.movement).await.unwrap();
	let (mvt_health_tx, mvt_health_rx) = tokio::sync::mpsc::channel(10);
	let two_stream =
		MovementMonitoring::build(&bridge_config.movement, mvt_health_rx).await.unwrap();

	let one_client_for_grpc = one_client.clone();

	// Initialize the gRPC health check service
	let health_service = HealthCheckService::default();
	health_service.set_service_status("", ServingStatus::Serving);
	health_service.set_service_status("Bridge", ServingStatus::Serving);

	let grpc_addr: SocketAddr = format!(
		"{}:{}",
		bridge_config.movement.grpc_listener_hostname, bridge_config.movement.grpc_port
	)
	.parse()
	.unwrap();

	let grpc_jh = tokio::spawn(async move {
		Server::builder()
			.add_service(HealthServer::new(health_service))
			.add_service(BridgeServer::new(one_client_for_grpc))
			.serve(grpc_addr)
			.await
	});

	// Initialize the gRPC health check service
	let (health_tx, health_rx) = tokio::sync::mpsc::channel(10);
	// Start the gRPC server on a specific address (e.g., localhost:50051)
	// Create and run the REST service
	let rest_service = BridgeRest::new(&bridge_config.movement, health_tx)?;
	let rest_service_future = rest_service.run_service();
	let rest_jh = tokio::spawn(rest_service_future);

	tracing::info!("Bridge Eth and Movement Inited. Starting bridge loop.");
	let indexer_db_client = match Client::from_env() {
		Ok(mut client) => {
			client.run_migrations()?;
			Some(client)
		}
		Err(e) => {
			tracing::warn!("Failed to create indexer db client: {e:?}");
			None
		}
	};

	let loop_jh = tokio::spawn(async move {
		bridge_service::run_bridge(
			one_client,
			one_stream,
			two_client,
			two_stream,
			health_rx,
			indexer_db_client,
			eth_health_tx,
			mvt_health_tx,
		)
		.await
	});

	tokio::select! {
		res = rest_jh => {
			tracing::error!("Heath check Rest server exit because :{res:?}");
		}
		res = loop_jh => {
			tracing::error!("Main relayer loop exit because :{res:?}");
		}
		res = grpc_jh => {
			tracing::error!("gRpc server exit because :{res:?}");
		}
	};

	Ok(())
}
