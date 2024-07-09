//! This crate provides configuration parameters for the MCR settlement
//! component of a Movement node.
use serde::{Deserialize, Serialize};
pub mod run_local;
pub mod deploy_remote;
pub mod common;	
use godfig::env_default;


#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {

	/// The ETH connection configuration.
	/// This is mandatory for all possible operations.
	#[serde(default)]
	pub eth_connection : common::eth_connection::Config,

	/// Whether or not to attempt to run locally.
	#[serde(default = "default_maybe_run_local")]
	pub maybe_run_local : bool,

	/// Optional deployment of contracts config
	
}

env_default!(
	default_maybe_run_local,
	"MAYBE_RUN_LOCAL",
	bool,
	false
);