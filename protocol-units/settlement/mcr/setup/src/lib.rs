use dot_movement::DotMovement;
use mcr_settlement_config::Config;

use std::future::Future;

pub mod local;
pub mod deploy_remote;

pub use local::Local;

/// Abstraction trait for MCR settlement setup strategies.
pub trait Setup {
	/// Sets up the MCR settlement client configuration.
	/// If required configuration values are unset, fills them with
	/// values decided by this setup strategy.
	fn setup(
		&self,
		dot_movement: &DotMovement,
		config: Config,
	) -> impl Future<Output = Result<(Config, tokio::task::JoinHandle<Result<String, anyhow::Error>>), anyhow::Error>> + Send;
}
