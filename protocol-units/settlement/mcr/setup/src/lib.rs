use dot_movement::DotMovement;
use mcr_settlement_config::Config;

use std::future::Future;

pub mod local;

/// Abstraction trait for MCR settlement setup strategies.
pub trait Setup {
	/// Sets up the MCR settlement client configuration.
	/// If required configuration values are unset, fills them with
	/// values decided by this setup strategy.
	fn setup(
		&self,
		dot_movement: &DotMovement,
		config: Config,
	) -> impl Future<Output = Result<Config, anyhow::Error>> + Send;
}
