use super::common::{
	default_gc_slot_duration_ms, default_ingress_account_whitelist, default_sequence_number_ttl_ms,
};
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

	/// The whitelist (path) for the mempool
	#[serde(default = "default_ingress_account_whitelist")]
	pub ingress_account_whitelist: Option<String>,
}

impl Default for Config {
	fn default() -> Self {
		Self {
			sequence_number_ttl_ms: default_sequence_number_ttl_ms(),
			gc_slot_duration_ms: default_gc_slot_duration_ms(),
			ingress_account_whitelist: default_ingress_account_whitelist(),
		}
	}
}

impl Config {
	pub fn whitelisted_accounts(&self) -> Result<Option<HashSet<AccountAddress>>, anyhow::Error> {
		match &self.ingress_account_whitelist {
			Some(whitelist_path) => {
				let mut whitelisted = HashSet::new();

				// read the file from memory
				let file_string = String::from_utf8(std::fs::read(whitelist_path)?)?;

				// for each line
				for line in file_string.lines() {
					let account = AccountAddress::from_hex(line.trim())?;
					whitelisted.insert(account);
				}

				Ok(Some(whitelisted))
			}
			None => Ok(None),
		}
	}
}
