use anyhow::Context;
use celestia_rpc::Client;
use celestia_types::nmt::Namespace;
use dot_movement::DotMovement;
use m1_da_light_node_grpc::*;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tracing::info;

/// The inner configuration for the local Celestia Appd Runner
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Config {
	/// The URL of the Celestia RPC
	#[serde(default = "Config::default_celestia_appd_rpc_connection_hostname")]
	pub celestia_appd_rpc_connection_hostname: Option<String>,

	/// The port of the Celestia RPC
	#[serde(default = "Config::default_celestia_appd_rpc_connection_port")]
	pub celestia_appd_rpc_connection_port: Option<u16>,

	/// The hostname of the Celestia Node websocket
	#[serde(default = "Config::default_celestia_appd_websocket_connection_hostname")]
	pub celestia_appd_websocket_connection_hostname: Option<String>,

	/// The port of the Celestia Node websocket
	#[serde(default = "Config::default_celestia_appd_websocket_connection_port")]
	pub celestia_appd_websocket_connection_port: Option<u16>,

	/// The hostname to listen on for the m1-da-light-node service
	#[serde(default = "Config::default_m1_da_light_node_listen_hostname")]
	pub m1_da_light_node_listen_hostname: Option<String>,

	/// The port to listen on for the m1-da-light-node service
	#[serde(default = "Config::default_m1_da_light_node_listen_port")]
	pub m1_da_light_node_listen_port: Option<u16>,
}

impl Config {
	/// The default hostname for the Celestia RPC
	pub fn default_celestia_appd_rpc_connection_hostname() -> Option<String> {
		Some("0.0.0.0".to_string())
	}

	/// The default port for the Celestia RPC
	pub fn default_celestia_appd_rpc_connection_port() -> Option<u16> {
		Some(26657)
	}

	/// The default hostname for the Celestia Node websocket
	pub fn default_celestia_appd_websocket_connection_hostname() -> Option<String> {
		Some("0.0.0.0".to_string())
	}

	/// The default port for the Celestia Node websocket
	pub fn default_celestia_appd_websocket_connection_port() -> Option<u16> {
		Some(26658)
	}

	/// The default hostname to listen on for the m1-da-light-node service
	pub fn default_m1_da_light_node_listen_hostname() -> Option<String> {
		Some("0.0.0.0".to_string())
	}

	/// The default port to listen on for the m1-da-light-node service
	pub fn default_m1_da_light_node_listen_port() -> Option<u16> {
		Some(30730)
	}
}

impl Default for Config {
	fn default() -> Self {
		Self {
			celestia_appd_rpc_connection_hostname:
				Config::default_celestia_appd_rpc_connection_hostname(),
			celestia_appd_rpc_connection_port: Config::default_celestia_appd_rpc_connection_port(),
			celestia_appd_websocket_connection_hostname:
				Config::default_celestia_appd_websocket_connection_hostname(),
			celestia_appd_websocket_connection_port:
				Config::default_celestia_appd_websocket_connection_port(),
			m1_da_light_node_listen_hostname: Config::default_m1_da_light_node_listen_hostname(),
			m1_da_light_node_listen_port: Config::default_m1_da_light_node_listen_port(),
		}
	}
}
