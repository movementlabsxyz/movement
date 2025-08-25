#[tokio::main]
async fn main() -> anyhow::Result<()> {
	use clap::Parser;
	use tracing_subscriber::EnvFilter;

	tracing_subscriber::fmt()
		.with_env_filter(
			EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
		)
		.init();

	replay_blocks::ApiReplayTool::parse().run().await
}
