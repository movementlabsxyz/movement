//! This crate provides configuration parameters for the MCR settlement
//! component of a Movement node.
use serde::{Deserialize, Serialize};
pub mod local;
pub mod deploy_remote;
pub mod common;	


#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Config {
	Local(local::Config),
	DeployRemote(deploy_remote::Config),
}