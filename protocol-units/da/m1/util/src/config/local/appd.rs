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
	#[serde(default = "Config::default_celestia_appd_rpc_listen_hostname")]
	pub celestia_appd_rpc_listen_hostname: Option<String>,

	/// The port of the Celestia RPC
	#[serde(default = "Config::default_celestia_appd_rpc_listen_port")]
	pub celestia_appd_rpc_listen_port: Option<u16>,

	/// The hostname of the Celestia Node websocket
	#[serde(default = "Config::default_celestia_appd_websocket_listen_hostname")]
	pub celestia_appd_websocket_listen_hostname: Option<String>,

	/// The port of the Celestia Node websocket
	#[serde(default = "Config::default_celestia_appd_websocket_listen_port")]
	pub celestia_appd_websocket_listen_port: Option<u16>,

	/// The auth token for the Celestia node
	pub celestia_appd_auth_token: Option<String>,

	/// The namespace for the Celestia node
	#[serde(default = "Config::default_namespace")]
	pub celestia_appd_namespace: Option<Namespace>,

	/// The celestia app path for when that is being orchestrated locally
	/// This does not have a default because if it is needed, a default is generally not appropriate.
	pub celestia_appd_path: Option<String>,

	/// The celestia validator address for when that is being orchestrated locally
	/// This does not have a default because if it is needed, a default is generally not appropriate.
	pub celestia_validator_address: Option<String>,
}

impl Config {
	/// The default hostname for the Celestia RPC
	pub fn default_celestia_appd_rpc_listen_hostname() -> Option<String> {
		Some("0.0.0.0".to_string())
	}

	/// The default port for the Celestia RPC
	pub fn default_celestia_appd_rpc_listen_port() -> Option<u16> {
		Some(26657)
	}

	/// The default hostname for the Celestia Node websocket
	pub fn default_celestia_appd_websocket_listen_hostname() -> Option<String> {
		Some("0.0.0.0".to_string())
	}

	/// The default port for the Celestia Node websocket
	pub fn default_celestia_appd_websocket_listen_port() -> Option<u16> {
		Some(26658)
	}

	/// The default namespace for the Celestia Node
	pub fn default_namespace() -> Option<Namespace> {
		Some(Namespace::new(0, b"default"))
	}
}

impl Default for Config {
	fn default() -> Self {
		Self {
			celestia_appd_rpc_listen_hostname: Config::default_celestia_appd_rpc_listen_hostname(),
			celestia_appd_rpc_listen_port: Config::default_celestia_appd_rpc_listen_port(),
			celestia_appd_websocket_listen_hostname:
				Config::default_celestia_appd_websocket_listen_hostname(),
			celestia_appd_websocket_listen_sport:
				Config::default_celestia_appd_websocket_listen_port(),
			celestia_appd_auth_token: None,
			celestia_appd_namespace: Config::default_namespace(),
			celestia_appd_path: None,
			celestia_validator_address: None,
		}
	}
}
