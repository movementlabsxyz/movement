use godfig::env_default;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DaSequencerConfig {
	/// The hostname to listen on for the movement-celestia-da-light-node service
	#[serde(default = "default_movement_da_sequencer_listen_address")]
	pub movement_da_sequencer_listen_address: SocketAddr,
}

// The default M1 DA Light Node listen hostname
env_default!(
	default_movement_da_sequencer_listen_address,
	"MOVEMENT_DA_SEQUENCER_LISTEN_ADDRESS",
	SocketAddr,
	"0.0.0.0:30730"
		.parse::<SocketAddr>()
		.expect("Bad da sequencer listener address.")
);

impl Default for DaSequencerConfig {
	fn default() -> Self {
		Self {
			movement_da_sequencer_listen_address: default_movement_da_sequencer_listen_address(),
		}
	}
}
