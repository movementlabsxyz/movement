pub mod path;

#[derive(Debug, Clone)]
pub struct DotMovement(std::path::PathBuf);

impl DotMovement {
	const DEFAULT_DOT_MOVEMENT_PATH_VAR_NAME: &'static str = "DOT_MOVEMENT_PATH";

	pub fn new(path: &str) -> Self {
		Self(std::path::PathBuf::from(path))
	}

	pub fn path(&self) -> &std::path::Path {
		&self.0
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
		assert_eq!(path.path(), std::path::Path::new("/tmp"));
	}

	#[test]
	fn test_try_from_env() -> Result<(), anyhow::Error> {
		std::env::set_var("DOT_MOVEMENT_PATH", "/tmp");
		let path = DotMovement::try_from_env()?;
		assert_eq!(path.path(), std::path::Path::new("/tmp"));
		Ok(())
	}
}
