use crate::config::common::default_propagate_execution_state;
use crate::config::common::default_stream_heartbeat_interval_sec;
use crate::config::common::{default_batch_signer_identifier, default_da_sequencer_connection_url};
use movement_signer_loader::identifiers::SignerIdentifier;
use serde::{Deserialize, Serialize};
use url::Url;

/// Configuration for the DA Sequencer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
	/// The number of milliseconds a sequence number is valid for.
	#[serde(default = "default_da_sequencer_connection_url")]
	pub connection_url: Url,

	/// The signing key used to sign batches.
	#[serde(default = "default_batch_signer_identifier")]
	pub batch_signer_identifier: SignerIdentifier,

	#[serde(default = "default_stream_heartbeat_interval_sec")]
	pub stream_heartbeat_interval_sec: u64,

	#[serde(default = "default_propagate_execution_state")]
	pub propagate_execution_state: bool,
}

impl Default for Config {
	fn default() -> Self {
		Self {
			connection_url: default_da_sequencer_connection_url(),
			batch_signer_identifier: default_batch_signer_identifier(),
			stream_heartbeat_interval_sec: default_stream_heartbeat_interval_sec(),
			propagate_execution_state: default_propagate_execution_state(),
		}
	}
}
