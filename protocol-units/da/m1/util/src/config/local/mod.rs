pub mod appd;
pub mod bridge;
pub mod m1_da_light_node;
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
}
