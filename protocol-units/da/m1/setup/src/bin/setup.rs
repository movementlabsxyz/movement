use m1_da_light_node_setup::{local::Local, Setup};

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
	use tracing_subscriber::EnvFilter;

	tracing_subscriber::fmt()
		.with_env_filter(
			EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
		)
		.init();

	let dot_movement = dot_movement::DotMovement::try_from_env()?;
	let path = dot_movement.path().join("config.toml");
	let config = m1_da_light_node_util::Config::try_from_toml_file(&path).unwrap_or_default();

	let local = Local::new();
	let config = local.setup(&dot_movement, config).await?;
	config.try_write_to_toml_file(&path)?;

	Ok(())
}
