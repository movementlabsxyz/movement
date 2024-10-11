pub mod appd;
pub mod bridge;
pub mod m1_da_light_node;
use crate::config::common::{
	default_celestia_force_new_chain, default_m1_da_light_node_is_initial,
};
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

	/// The m1-da-light-node configuration
	#[serde(default)]
	pub m1_da_light_node: m1_da_light_node::Config,

	/// Whether to force a new chain
	#[serde(default = "default_celestia_force_new_chain")]
	pub celestia_force_new_chain: bool,

	/// The memseq configuration
	#[serde(default)]
	pub memseq: MemseqConfig,

	#[serde(default = "default_m1_da_light_node_is_initial")]
	pub m1_da_light_node_is_initial: bool,
}

impl Default for Config {
	fn default() -> Self {
		Self {
			appd: appd::Config::default(),
			bridge: bridge::Config::default(),
			m1_da_light_node: m1_da_light_node::Config::default(),
			celestia_force_new_chain: default_celestia_force_new_chain(),
			memseq: MemseqConfig::default(),
			m1_da_light_node_is_initial: default_m1_da_light_node_is_initial(),
		}
	}
}
