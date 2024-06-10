pub mod path;

#[derive(Debug)]
pub struct DotMovementPath(std::path::PathBuf);

impl DotMovementPath {

    const DEFAULT_DOT_MOVEMENT_PATH_VAR_NAME: &'static str = "DOT_MOVEMENT_PATH";

    pub fn new(path: &str) -> Self {
        Self(std::path::PathBuf::from(path))
    }

    pub fn get_path(&self) -> &std::path::Path {
        &self.0
    }

    pub fn try_from_env() -> Result<Self, anyhow::Error> {
        let path = std::env::var(Self::DEFAULT_DOT_MOVEMENT_PATH_VAR_NAME)
            .map_err(|_| anyhow::anyhow!("Dot movement path not provided"))?;
        Ok(Self::new(&path))
    }

}

impl Into<std::path::PathBuf> for DotMovementPath {
    fn into(self) -> std::path::PathBuf {
        self.0
    }
}

#[cfg(test)]
pub mod test {
    use super::*;

    #[test]
    fn test_dot_movement_path() {
        let path = DotMovementPath::new("/tmp");
        assert_eq!(path.get_path(), std::path::Path::new("/tmp"));
    }

    #[test]
    fn test_try_from_env() -> Result<(), anyhow::Error> {
        std::env::set_var("DOT_MOVEMENT_PATH", "/tmp");
        let path = DotMovementPath::try_from_env()?;
        assert_eq!(path.get_path(), std::path::Path::new("/tmp"));
        Ok(())
    }

}