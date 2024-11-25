use super::common::{
	default_gc_slot_duration_ms, default_ingress_account_whitelist, default_sequence_number_ttl_ms,
};
use aptos_account_whitelist::file::{Whitelist, WhitelistOperations};
use aptos_types::account_address::AccountAddress;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
	/// The number of milliseconds a sequence number is valid for.
	#[serde(default = "default_sequence_number_ttl_ms")]
	pub sequence_number_ttl_ms: u64,

	/// The duration of a garbage collection slot in milliseconds.
	#[serde(default = "default_gc_slot_duration_ms")]
	pub gc_slot_duration_ms: u64,
}

impl Default for Config {
	fn default() -> Self {
		Self {
			sequence_number_ttl_ms: default_sequence_number_ttl_ms(),
			gc_slot_duration_ms: default_gc_slot_duration_ms(),
		}
	}
}
