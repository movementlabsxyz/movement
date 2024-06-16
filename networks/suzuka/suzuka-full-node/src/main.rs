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
	let path = dot_movement.get_path().join("config.toml");
	let config = suzuka_config::Config::try_from_toml_file(&path).unwrap_or_default();
	let (executor, background_task) = SuzukaPartialNode::try_from_config(config)
		.await
		.context("Failed to create the executor")?;

	tokio::spawn(background_task);

	executor.run().await.context("Failed to run suzuka")?;

	Ok(())
}
