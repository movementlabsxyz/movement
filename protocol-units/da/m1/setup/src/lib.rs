use dot_movement::DotMovement;
use m1_da_light_node_util::Config;

use std::future::Future;

pub mod local;

pub trait Setup {
	fn setup(
		&self,
		dot_movement: &DotMovement,
		config: Config,
	) -> impl Future<Output = Result<Config, anyhow::Error>> + Send;
}
