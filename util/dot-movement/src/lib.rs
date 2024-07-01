pub mod path;

#[derive(Debug, Clone)]
pub struct DotMovement(std::path::PathBuf);

impl DotMovement {
	const DEFAULT_DOT_MOVEMENT_PATH_VAR_NAME: &'static str = "DOT_MOVEMENT_PATH";

	pub fn new(path: &str) -> Self {
		Self(std::path::PathBuf::from(path))
	}

	pub fn get_path(&self) -> &std::path::Path {
		&self.0
	}

	pub fn get_config_json_path(&self) -> std::path::PathBuf {
		self.0.join("config.json")
	}

	/// Tries to get a configuration from a JSON file.
	pub fn try_get_config_from_json<T: serde::de::DeserializeOwned>(
		&self,
	) -> Result<T, anyhow::Error> {
		let file = std::fs::File::open(self.get_config_json_path())
			.map_err(|e| anyhow::anyhow!("Failed to open file: {}", e))?;
		let reader = std::io::BufReader::new(file);
		let config = serde_json::from_reader(reader)
			.map_err(|e| anyhow::anyhow!("Failed to parse config: {}", e))?;
		Ok(config)
	}

	/// Tries to write a configuration to a JSON file.
	pub fn try_write_config_to_json<T: serde::Serialize>(
		&self,
		config: &T,
	) -> Result<(), anyhow::Error> {
		let file = std::fs::File::create(self.get_config_json_path())
			.map_err(|e| anyhow::anyhow!("Failed to create file: {}", e))?;
		let writer = std::io::BufWriter::new(file);
		serde_json::to_writer_pretty(writer, config)
			.map_err(|e| anyhow::anyhow!("Failed to write config: {}", e))?;
		Ok(())
	}

	pub fn try_from_env() -> Result<Self, anyhow::Error> {
		let path = std::env::var(Self::DEFAULT_DOT_MOVEMENT_PATH_VAR_NAME)
			.map_err(|_| anyhow::anyhow!("Dot movement path not provided"))?;
		Ok(Self::new(&path))
	}
}

impl Into<std::path::PathBuf> for DotMovement {
	fn into(self) -> std::path::PathBuf {
		self.0
	}
}

#[cfg(test)]
pub mod test {
	use super::*;

	#[test]
	fn test_dot_movement_path() {
		let path = DotMovement::new("/tmp");
		assert_eq!(path.get_path(), std::path::Path::new("/tmp"));
	}

	#[test]
	fn test_try_from_env() -> Result<(), anyhow::Error> {
		std::env::set_var("DOT_MOVEMENT_PATH", "/tmp");
		let path = DotMovement::try_from_env()?;
		assert_eq!(path.get_path(), std::path::Path::new("/tmp"));
		Ok(())
	}
}
