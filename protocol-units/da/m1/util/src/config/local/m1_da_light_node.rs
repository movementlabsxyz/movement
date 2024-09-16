use crate::config::common::{
	default_celestia_rpc_connection_hostname, default_celestia_rpc_connection_port,
	default_celestia_websocket_connection_hostname, default_celestia_websocket_connection_port,
	default_da_signing_private_key, default_m1_da_light_node_connection_hostname,
	default_m1_da_light_node_connection_port, default_m1_da_light_node_listen_hostname,
	default_m1_da_light_node_listen_port,
};
use serde::{Deserialize, Serialize};

/// The inner configuration for the local Celestia Appd Runner
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Config {
	/// The URL of the Celestia RPC
	#[serde(default = "default_celestia_rpc_connection_hostname")]
	pub celestia_rpc_connection_hostname: String,

	/// The port of the Celestia RPC
	#[serde(default = "default_celestia_rpc_connection_port")]
	pub celestia_rpc_connection_port: u16,

	/// The hostname of the Celestia Node websocket
	#[serde(default = "default_celestia_websocket_connection_hostname")]
	pub celestia_websocket_connection_hostname: String,

	/// The port of the Celestia Node websocket
	#[serde(default = "default_celestia_websocket_connection_port")]
	pub celestia_websocket_connection_port: u16,

	/// The hostname to listen on for the m1-da-light-node service
	#[serde(default = "default_m1_da_light_node_listen_hostname")]
	pub m1_da_light_node_listen_hostname: String,

	/// The port to listen on for the m1-da-light-node service
	#[serde(default = "default_m1_da_light_node_listen_port")]
	pub m1_da_light_node_listen_port: u16,

	/// The hostname for m1-da-light-node connection
	#[serde(default = "default_m1_da_light_node_connection_hostname")]
	pub m1_da_light_node_connection_hostname: String,

	/// The port for m1-da-light-node connection
	#[serde(default = "default_m1_da_light_node_connection_port")]
	pub m1_da_light_node_connection_port: u16,

	/// The private key for signing DA messages
	#[serde(default = "default_da_signing_private_key")]
	pub da_signing_private_key: String,
}

impl Default for Config {
	fn default() -> Self {
		Self {
			celestia_rpc_connection_hostname: default_celestia_rpc_connection_hostname(),
			celestia_rpc_connection_port: default_celestia_rpc_connection_port(),
			celestia_websocket_connection_hostname: default_celestia_websocket_connection_hostname(
			),
			celestia_websocket_connection_port: default_celestia_websocket_connection_port(),
			m1_da_light_node_listen_hostname: default_m1_da_light_node_listen_hostname(),
			m1_da_light_node_listen_port: default_m1_da_light_node_listen_port(),
			m1_da_light_node_connection_hostname: default_m1_da_light_node_connection_hostname(),
			m1_da_light_node_connection_port: default_m1_da_light_node_connection_port(),
			da_signing_private_key: default_da_signing_private_key(),
		}
	}
}
