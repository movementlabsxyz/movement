pub mod local;
use std::future::Future;

pub trait MovementFullNodeSetupOperations {
	fn setup(
		&self,
		dot_movement: dot_movement::DotMovement,
		config: movement_config::Config,
	) -> impl Future<
		Output = Result<
			(movement_config::Config, tokio::task::JoinHandle<Result<String, anyhow::Error>>),
			anyhow::Error,
		>,
	>;
}
