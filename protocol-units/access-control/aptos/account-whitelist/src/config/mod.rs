use crate::file::{Whitelist, WhitelistOperations};
use aptos_types::account_address::AccountAddress;
use godfig::env_default;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

env_default!(default_aptos_account_whitelist, "APTOS_ACCOUNT_WHITELIST", String);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
	/// The whitelist (path) for the mempool
	#[serde(default = "default_aptos_account_whitelist")]
	pub ingress_account_whitelist: Option<String>,
}

impl Default for Config {
	fn default() -> Self {
		Self { ingress_account_whitelist: default_aptos_account_whitelist() }
	}
}

impl Config {
	pub fn whitelisted_accounts(&self) -> Result<Option<HashSet<AccountAddress>>, anyhow::Error> {
		match &self.ingress_account_whitelist {
			Some(whitelist_path) => {
				let whitelist = Whitelist::try_new(whitelist_path.as_str())?;
				let whitelisted = whitelist.try_into_set()?;

				// convert into inner
				let whitelisted = whitelisted
					.into_iter()
					.map(|whitelisted| whitelisted.into_inner())
					.collect::<HashSet<AccountAddress>>();

				Ok(Some(whitelisted))
			}
			None => Ok(None),
		}
	}
}
