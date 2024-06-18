use dot_movement::DotMovement;
use suzuka_config::Config;

pub mod local;

pub trait SuzukaFullNodeSetupOperations {
	async fn setup(
		&self,
		dot_movement: &DotMovement,
		config: Config,
	) -> Result<Config, anyhow::Error>;
}
