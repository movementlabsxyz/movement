use super::common::{
    default_maptos_indexer_grpc_listen_hostname, default_maptos_indexer_grpc_listen_port,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {

    /// The URL of the Aptos gRPC indexer server
    #[serde(default = "default_maptos_indexer_grpc_listen_hostname")]
    pub maptos_indexer_grpc_listen_hostname: String,

    /// The port of the Aptos gRPC indexer server
    #[serde(default = "default_maptos_indexer_grpc_listen_port")]
    pub maptos_indexer_grpc_listen_port: u16,
}

impl Default for Config {
	fn default() -> Self {
		Self {
            maptos_indexer_grpc_listen_hostname: default_maptos_indexer_grpc_listen_hostname(),
            maptos_indexer_grpc_listen_port: default_maptos_indexer_grpc_listen_port(),
		}
	}
}
