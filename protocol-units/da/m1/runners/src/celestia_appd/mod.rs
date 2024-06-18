use crate::Runner;
use dot_movement::DotMovement;
use m1_da_light_node_util::Config;

pub mod local;

#[derive(Debug, Clone)]
pub enum CelestiaAppd {
	Local(local::Local),
}

impl CelestiaAppd {
	pub fn local() -> Self {
		CelestiaAppd::Local(local::Local::new())
	}
}

impl Runner for CelestiaAppd {
	async fn run(&self, dot_movement: &DotMovement, config: Config) -> Result<(), anyhow::Error> {
		match self {
			CelestiaAppd::Local(local) => local.run(dot_movement, config).await,
		}
	}
}
