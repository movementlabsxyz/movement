use base64::prelude::*;
use celestia_types::nmt::Namespace;
use godfig::env_default;

use std::env;
use std::path::PathBuf;

// The default hostname for the Celestia RPC
env_default!(
	default_celestia_rpc_listen_hostname,
	"CELESTIA_RPC_LISTEN_HOSTNAME",
	String,
	"0.0.0.0".to_string()
);

// The default port for the Celestia RPC
env_default!(default_celestia_rpc_listen_port, "CELESTIA_RPC_LISTEN_PORT", u16, 26657);

// The default Celestia RPC connection protocol
env_default!(
	default_celestia_rpc_connection_protocol,
	"CELESTIA_RPC_CONNECTION_PROTOCOL",
	String,
	"http".to_string()
);

// The default Celestia RPC connection hostname
env_default!(
	default_celestia_rpc_connection_hostname,
	"CELESTIA_RPC_CONNECTION_HOSTNAME",
	String,
	"0.0.0.0".to_string()
);

// The default Celestia RPC connection port
env_default!(default_celestia_rpc_connection_port, "CELESTIA_RPC_CONNECTION_PORT", u16, 26657);

// The default hostname for the Celestia Node websocket
env_default!(
	default_celestia_websocket_listen_hostname,
	"CELESTIA_WEBSOCKET_LISTEN_HOSTNAME",
	String,
	"0.0.0.0".to_string()
);

// The default port for the Celestia Node websocket
env_default!(default_celestia_websocket_listen_port, "CELESTIA_WEBSOCKET_LISTEN_PORT", u16, 26658);

// the default Celestia Node websocket connection protocol
env_default!(
	default_celestia_websocket_connection_protocol,
	"CELESTIA_WEBSOCKET_CONNECTION_PROTOCOL",
	String,
	"ws".to_string()
);

// The default Celestia Node websocket connection hostname
env_default!(
	default_celestia_websocket_connection_hostname,
	"CELESTIA_WEBSOCKET_CONNECTION_HOSTNAME",
	String,
	"0.0.0.0".to_string()
);

// The default Celestia Node websocket connection port
env_default!(
	default_celestia_websocket_connection_port,
	"CELESTIA_WEBSOCKET_CONNECTION_PORT",
	u16,
	26658
);

// The default Celestia Node websocket path
env_default!(
	default_celestia_websocket_connection_path,
	"CELESTIA_WEBSOCKET_CONNECTION_PATH",
	String,
	"".to_string()
);

// The default M1 DA Light Node listen hostname
env_default!(
	default_movement_da_light_node_listen_hostname,
	"MOVEMENT_DA_LIGHT_NODE_LISTEN_HOSTNAME",
	String,
	"0.0.0.0".to_string()
);

// The default M1 DA Light Node listen port
env_default!(
	default_movement_da_light_node_listen_port,
	"MOVEMENT_DA_LIGHT_NODE_LISTEN_PORT",
	u16,
	30730
);

// The default M1 DA Light Node connection protocol
env_default!(
	default_movement_da_light_node_connection_protocol,
	"MOVEMENT_DA_LIGHT_NODE_CONNECTION_PROTOCOL",
	String,
	"http".to_string()
);

// The default M1 DA Light Node connection hostname
env_default!(
	default_movement_da_light_node_connection_hostname,
	"MOVEMENT_DA_LIGHT_NODE_CONNECTION_HOSTNAME",
	String,
	"0.0.0.0".to_string()
);

// The default M1 DA Light Node connection port
env_default!(
	default_movement_da_light_node_connection_port,
	"MOVEMENT_DA_LIGHT_NODE_CONNECTION_PORT",
	u16,
	30730
);

// The default Celestia Namespace
pub fn default_celestia_namespace() -> Namespace {
	match env::var("CELESTIA_NAMESPACE") {
		Ok(val) => {
			if val.starts_with("0x") {
				let id = hex::decode(&val[2..]).expect("failed to decode hexadecimal namespace ID");
				Namespace::new_v0(&id).expect("invalid namespace ID")
			} else {
				let id =
					BASE64_STANDARD.decode(&val).expect("failed to decode base64 namespace ID");
				Namespace::from_raw(&id).expect("invalid namespace ID")
			}
		}
		// todo: get rid of this unwrap somehow, even though it should never fail
		Err(env::VarError::NotPresent) => Namespace::new_v0(b"movement").unwrap(),
		Err(e) => panic!("invalid environment value of CELESTIA_NAMESPACE: {e}"),
	}
}

// The default Celestia chain id
env_default!(default_celestia_chain_id, "CELESTIA_CHAIN_ID", String, "movement".to_string());

// Whether to force a new chain
env_default!(default_celestia_force_new_chain, "CELESTIA_FORCE_NEW_CHAIN", bool, true);

// Whether to use replace args for Celestia appd
env_default!(default_celestia_appd_use_replace_args, "CELESTIA_USE_REPLACE_ARGS", bool, false);

/// The replacement args for Celestia appd
pub fn default_celestia_appd_replace_args() -> Vec<String> {
	match std::env::var("CELESTIA_REPLACE_ARGS") {
		Ok(val) => val.split(',').map(|s| s.to_string()).collect(),
		Err(_) => vec![],
	}
}

// Whether to use replace args for Celestia bridge
env_default!(
	default_celestia_bridge_use_replace_args,
	"CELESTIA_BRIDGE_USE_REPLACE_ARGS",
	bool,
	false
);

/// The replacement args for Celestia bridge
pub fn default_celestia_bridge_replace_args() -> Vec<String> {
	match std::env::var("CELESTIA_BRIDGE_REPLACE_ARGS") {
		Ok(val) => val.split(',').map(|s| s.to_string()).collect(),
		Err(_) => vec![],
	}
}

// Whether to use replace args for Celestia bridge
env_default!(default_da_light_node_is_initial, "MOVEMENT_DA_LIGHT_NODE_IS_INITIAL", bool, true);

// Whether to use http1 for Movement Light Node Connections
env_default!(default_movement_da_light_node_http1, "MOVEMENT_DA_LIGHT_NODE_HTTP1", bool, false);

// The Celestia light node's signing keyname
env_default!(
	default_celestia_light_node_key_name,
	"CELESTIA_LIGHT_KEYNAME",
	String,
	"movement_celestia_light".into()
);

pub fn default_celestia_light_node_store() -> Option<PathBuf> {
	std::env::var_os("CELESTIA_LIGHT_NODE_STORE").map(Into::into)
}
