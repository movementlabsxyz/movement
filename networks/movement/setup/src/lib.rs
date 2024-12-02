pub mod local;

pub trait MovementFullNodeSetupOperations {
	async fn setup(
		&self,
		dot_movement: dot_movement::DotMovement,
		config: movement_config::Config,
	) -> Result<
		(movement_config::Config, tokio::task::JoinHandle<Result<String, anyhow::Error>>),
		anyhow::Error,
	>;
}
