use suzuka_full_node::{
	manager::Manager,
	partial::SuzukaPartialNode
};
use maptos_dof_execution::v1::Executor;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
	
	use tracing_subscriber::EnvFilter;

	tracing_subscriber::fmt()
		.with_env_filter(
			EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
		)
		.init();

	let dot_movement = dot_movement::DotMovement::try_from_env()?;
	let config_path = dot_movement.get_config_json_path();
	let config_file = tokio::fs::File::open(config_path).await?;
	let manager = Manager::<SuzukaPartialNode<Executor>>::new(config_file).await?;
	manager.try_run().await?;
	
	Ok(())
}
