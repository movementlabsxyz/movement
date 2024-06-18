use dot_movement::DotMovement;
use monza_config::Config;

pub mod local;

pub trait MonzaFullNodeSetupOperations {
	async fn setup(
		&self,
		dot_movement: &DotMovement,
		config: Config,
	) -> Result<Config, anyhow::Error>;
}
