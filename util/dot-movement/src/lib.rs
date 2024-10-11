use std::io::Write;
pub mod path;
pub mod sync;

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

	pub fn set_path(&mut self, path: std::path::PathBuf) {
		self.0 = path;
	}

	pub fn get_config_json_path(&self) -> std::path::PathBuf {
		self.0.join("config.json")
	}

	pub async fn try_get_or_create_config_file(&self) -> Result<tokio::fs::File, anyhow::Error> {
		let config_path = self.get_config_json_path();

		// get res for opening in read-write mode
		let res = tokio::fs::OpenOptions::new()
			.read(true)
			.write(true)
			.open(config_path.clone())
			.await;

		match res {
			Ok(file) => Ok(file),
			Err(_e) => {
				// create parent directories
				tokio::fs::DirBuilder::new()
					.recursive(true)
					.create(
						config_path.parent().ok_or(anyhow::anyhow!(
							"Failed to get parent directory of config path"
						))?,
					)
					.await?;

				// create the file
				let file = tokio::fs::File::create_new(config_path).await?;

				Ok(file)
			}
		}
	}

	/// Tries to get a configuration from a JSON file.
	pub fn try_get_or_create_config_from_json<
		T: serde::de::DeserializeOwned + serde::ser::Serialize + Default,
	>(
		&self,
	) -> Result<T, anyhow::Error> {
		let config_path = self.get_config_json_path();
		// get res for opening in read-write mode
		let res = std::fs::OpenOptions::new().read(true).write(true).open(config_path.clone());

		let file = match res {
			Ok(file) => file,
			Err(_e) => {
				// create parent directories
				std::fs::DirBuilder::new().recursive(true).create(
					config_path
						.parent()
						.ok_or(anyhow::anyhow!("Failed to get parent directory of config path"))?,
				)?;

				// create the file
				{
					let mut file = std::fs::File::create_new(&config_path)?;
					let default_config = T::default();
					let json_contents = serde_json::to_string_pretty(&default_config)?;
					file.write_all(json_contents.as_bytes())?;
					file.sync_all()?;
				}
				std::fs::OpenOptions::new().read(true).write(true).open(config_path.clone())?
			}
		};
		let reader = std::io::BufReader::new(file);
		let config = serde_json::from_reader(reader)
			.map_err(|e| anyhow::anyhow!("Failed to parse config: {}", e))?;
		Ok(config)
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
