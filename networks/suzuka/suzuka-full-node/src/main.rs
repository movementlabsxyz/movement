use anyhow::Context;
use suzuka_full_node::{partial::SuzukaPartialNode, SuzukaFullNode};

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
	#[cfg(feature = "logging")]
	{
		use tracing_subscriber::EnvFilter;

		tracing_subscriber::fmt()
			.with_env_filter(
				EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
			)
			.init();
	}

	let dot_movement = dot_movement::DotMovement::try_from_env()?;
	let config = dot_movement.try_get_config_from_json::<suzuka_config::Config>()?;
	let (executor, background_task) = SuzukaPartialNode::try_from_config(config)
		.await
		.context("Failed to create the executor")?;

	tokio::spawn(background_task);

	executor.run().await.context("Failed to run suzuka")?;

	Ok(())
}
