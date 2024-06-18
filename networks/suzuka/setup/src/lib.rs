use dot_movement::DotMovement;
use suzuka_config::Config;

use std::future::Future;

pub mod local;

pub trait Setup {
	fn setup(
		&self,
		dot_movement: &DotMovement,
		config: Config,
	) -> impl Future<Output = Result<Config, anyhow::Error>> + Send;
}
