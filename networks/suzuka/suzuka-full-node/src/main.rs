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

	// Load variables defined in .env file.
	let movement_storage_path =
		std::env::var("MOVEMENT_BASE_STORAGE_PATH").unwrap_or("".to_string());
	let mut env_file_path = std::env::current_dir()?;
	env_file_path.push(movement_storage_path);
	env_file_path.push(".env".to_string());
	dotenv::from_filename(env_file_path)?;

	let (executor, background_task) = SuzukaPartialNode::try_from_env()
		.await
		.context("Failed to create the executor")?;

	tokio::spawn(background_task);

	executor.run().await.context("Failed to run suzuka")?;

	Ok(())
}
