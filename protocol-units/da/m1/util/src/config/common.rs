use celestia_types::nmt::Namespace;
use godfig::env_default;

// The default hostname for the Celestia RPC
env_default!(
	default_celestia_rpc_listen_hostname,
	"CELESTIA_RPC_LISTEN_HOSTNAME",
	String,
	"0.0.0.0".to_string()
);

// The default port for the Celestia RPC
env_default!(default_celestia_rpc_listen_port, "CELESTIA_RPC_LISTEN_PORT", u16, 26657);

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

// The default M1 DA Light Node listen hostname
env_default!(
	default_m1_da_light_node_listen_hostname,
	"M1_DA_LIGHT_NODE_LISTEN_HOSTNAME",
	String,
	"0.0.0.0".to_string()
);

// The default M1 DA Light Node listen port
env_default!(default_m1_da_light_node_listen_port, "M1_DA_LIGHT_NODE_LISTEN_PORT", u16, 30730);

// The default M1 DA Light Node connection hostname
env_default!(
	default_m1_da_light_node_connection_hostname,
	"M1_DA_LIGHT_NODE_CONNECTION_HOSTNAME",
	String,
	"0.0.0.0".to_string()
);

// The default M1 DA Light Node connection port
env_default!(
	default_m1_da_light_node_connection_port,
	"M1_DA_LIGHT_NODE_CONNECTION_PORT",
	u16,
	30730
);

// The default Celestia Namespace
pub fn default_celestia_namespace() -> Namespace {
	match std::env::var("CELESTIA_NAMESPACE") {
		Ok(val) => match serde_json::from_str(&val) {
			Ok(namespace) => namespace,
			// todo: get rid of this unwrap somehow, event though it should never fail
			Err(_) => Namespace::new_v0(b"movement").unwrap(),
		},
		// todo: get rid of this unwrap somehow, event though it should never fail
		Err(_) => Namespace::new_v0(b"movement").unwrap(),
	}
}

// The default Celestia chain id
env_default!(default_celestia_chain_id, "CELESTIA_CHAIN_ID", String, "movement".to_string());

// Whether to force a new chain
env_default!(default_celestia_force_new_chain, "CELESTIA_FORCE_NEW_CHAIN", bool, true);
