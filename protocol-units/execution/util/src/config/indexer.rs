use super::common::{
	default_maptos_indexer_grpc_inactivity_timeout, default_maptos_indexer_grpc_listen_hostname,
	default_maptos_indexer_grpc_listen_port, default_maptos_indexer_grpc_ping_interval,
	default_maptos_indexer_healthcheck_hostname, default_maptos_indexer_healthcheck_port,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
	/// The URL of the Suzuka node gRPC indexer server
	#[serde(default = "default_maptos_indexer_grpc_listen_hostname")]
	pub maptos_indexer_grpc_listen_hostname: String,

	/// The port of the Suzuka node gRPC indexer server
	#[serde(default = "default_maptos_indexer_grpc_listen_port")]
	pub maptos_indexer_grpc_listen_port: u16,

	/// Inactivity timeout of the gRpc connection
	#[serde(default = "default_maptos_indexer_grpc_inactivity_timeout")]
	pub maptos_indexer_grpc_inactivity_timeout: u64,

	/// Ping interval of the gRpc connection
	#[serde(default = "default_maptos_indexer_grpc_ping_interval")]
	pub maptos_indexer_grpc_inactivity_ping_interval: u64,

	/// The URL of the indexer health check entry point
	#[serde(default = "default_maptos_indexer_healthcheck_hostname")]
	pub maptos_indexer_grpc_healthcheck_hostname: String,

	/// The port of the indexer health check entry point
	#[serde(default = "default_maptos_indexer_healthcheck_port")]
	pub maptos_indexer_grpc_healthcheck_port: u16,
}

impl Default for Config {
	fn default() -> Self {
		Self {
			maptos_indexer_grpc_listen_hostname: default_maptos_indexer_grpc_listen_hostname(),
			maptos_indexer_grpc_listen_port: default_maptos_indexer_grpc_listen_port(),
			maptos_indexer_grpc_inactivity_timeout: default_maptos_indexer_grpc_inactivity_timeout(
			),
			maptos_indexer_grpc_inactivity_ping_interval: default_maptos_indexer_grpc_ping_interval(
			),
			maptos_indexer_grpc_healthcheck_hostname: default_maptos_indexer_healthcheck_hostname(),
			maptos_indexer_grpc_healthcheck_port: default_maptos_indexer_healthcheck_port(),
		}
	}
}
