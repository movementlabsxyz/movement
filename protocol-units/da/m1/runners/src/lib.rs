use dot_movement::DotMovement;
use m1_da_light_node_util::Config;

use std::future::Future;

pub mod celestia_appd;
pub mod celestia_bridge;

pub trait Runner {
	fn run(
		&self,
		dot_movement: &DotMovement,
		config: Config,
	) -> impl Future<Output = Result<(), anyhow::Error>> + Send;
}
