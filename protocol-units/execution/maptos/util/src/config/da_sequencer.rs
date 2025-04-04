use crate::config::common::{default_batch_signer_identifier, default_da_sequencer_connection_url};
use movement_signer_loader::identifiers::SignerIdentifier;
use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
	/// The number of milliseconds a sequence number is valid for.
	#[serde(default = "default_da_sequencer_connection_url")]
	pub connection_url: Url,

	/// The signing key used to sign batches.
	#[serde(skip_deserializing, skip_serializing, default = "default_batch_signer_identifier")]
	pub batch_signer_identifier: SignerIdentifier,
}

impl Default for Config {
	fn default() -> Self {
		Self {
			connection_url: default_da_sequencer_connection_url(),
			batch_signer_identifier: default_batch_signer_identifier(),
		}
	}
}
