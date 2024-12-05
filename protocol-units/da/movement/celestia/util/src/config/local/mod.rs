pub mod appd;
pub mod bridge;
pub mod da_light_node;
use crate::config::common::{default_celestia_force_new_chain, default_da_light_node_is_initial};
use aptos_account_whitelist::config::Config as WhitelistConfig;
use memseq_util::Config as MemseqConfig;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Config {
	/// The appd configuration
	#[serde(default)]
	pub appd: appd::Config,

	/// The bridge configuration
	#[serde(default)]
	pub bridge: bridge::Config,

	/// The movement-celestia-da-light-node configuration
	#[serde(default)]
	pub da_light_node: da_light_node::Config,

	/// Whether to force a new chain
	#[serde(default = "default_celestia_force_new_chain")]
	pub celestia_force_new_chain: bool,

	/// The memseq configuration
	#[serde(default)]
	pub memseq: MemseqConfig,

	#[serde(default = "default_da_light_node_is_initial")]
	pub da_light_node_is_initial: bool,

	/// The access control config
	#[serde(default)]
	pub access_control: WhitelistConfig,
}

impl Default for Config {
	fn default() -> Self {
		Self {
			appd: appd::Config::default(),
			bridge: bridge::Config::default(),
			da_light_node: da_light_node::Config::default(),
			celestia_force_new_chain: default_celestia_force_new_chain(),
			memseq: MemseqConfig::default(),
			da_light_node_is_initial: default_da_light_node_is_initial(),
			access_control: WhitelistConfig::default(),
		}
	}
}
