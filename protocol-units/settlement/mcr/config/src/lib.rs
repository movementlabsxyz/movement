//! This crate provides configuration parameters for the MCR settlement
//! component of a Movement node.
use serde::{Deserialize, Serialize};
pub mod run_local;
pub mod deploy_remote;
pub mod common;

use godfig::env_short_default;
use common::deploy::default_maybe_deploy;
use common::settlement::default_maybe_settle;


#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {

	/// The ETH connection configuration.
	/// This is mandatory for all possible operations.
	#[serde(default)]
	pub eth_connection : common::eth_connection::Config,

	#[serde(default = "default_maybe_settle")]
	pub settle : Option<common::settlement::Config>,

	/// Whether or not to attempt to run locally.
	#[serde(default = "default_maybe_run_local")]
	pub maybe_run_local : bool,

	/// Optional deployment of contracts config
	#[serde(default = "default_maybe_deploy")]
	pub maybe_deploy : Option<common::deploy::Config>,
	
}

env_short_default!(
	default_maybe_run_local,
	bool,
	false
);