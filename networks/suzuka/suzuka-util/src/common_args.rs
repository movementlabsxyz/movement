use clap::Parser;
use dot_movement::DotMovement;
use godfig::{backend::config_file::ConfigFile, Godfig};
use suzuka_config::Config;

/// A struct containing common arguments for the Suzuka network.
#[derive(Parser, Debug, Clone)]
pub struct MovementArgs {
	/// The optional path to the DOT_MOVEMENT directory.
	/// This will be read from an environment variable if not provided.
	#[clap(long, env = "DOT_MOVEMENT_PATH")]
	pub movement_path: Option<String>,
}

impl MovementArgs {
	/// Create a new instance of `MovementArgs`.
	pub fn new() -> Self {
		Self { movement_path: None }
	}

	/// Get the DotMovement struct from the args.
	pub fn dot_movement(&self) -> Result<DotMovement, anyhow::Error> {
		let movement_path = self.movement_path.clone().unwrap_or_else(|| {
			std::env::var("DOT_MOVEMENT_PATH").unwrap_or_else(|_| ".".to_string())
		});
		Ok(DotMovement::new(movement_path.as_str()))
	}

	/// Get the config
	pub async fn config(&self) -> Result<Config, anyhow::Error> {
		let dot_movement = self.dot_movement()?;
		let config_file = dot_movement.try_get_or_create_config_file().await?;
		let godfig: Godfig<Config, ConfigFile> = Godfig::new(ConfigFile::new(config_file), vec![]);

		godfig.try_wait_for_ready().await.map_err(|e| e.into())
	}
}
