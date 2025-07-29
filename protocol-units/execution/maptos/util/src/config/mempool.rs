use super::common::{default_gc_slot_duration_ms, default_sequence_number_ttl_ms};
use crate::config::common::{default_max_batch_size, default_max_tx_per_batch};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
	/// The number of milliseconds a sequence number is valid for.
	#[serde(default = "default_sequence_number_ttl_ms")]
	pub sequence_number_ttl_ms: u64,

	/// The duration of a garbage collection slot in milliseconds.
	#[serde(default = "default_gc_slot_duration_ms")]
	pub gc_slot_duration_ms: u64,

	/// Max number of Tx added per batch.
	#[serde(default = "default_max_tx_per_batch")]
	pub max_tx_per_batch: u64,

	/// Max batch size in bytes.
	#[serde(default = "default_max_batch_size")]
	pub max_batch_size: u64,
}

impl Default for Config {
	fn default() -> Self {
		Self {
			sequence_number_ttl_ms: default_sequence_number_ttl_ms(),
			gc_slot_duration_ms: default_gc_slot_duration_ms(),
			max_tx_per_batch: default_max_tx_per_batch(),
			max_batch_size: default_max_batch_size(),
		}
	}
}
