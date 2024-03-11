use util::{
    checker::Checker,
    artifact::Artifact,
    util::util::patterns::constructor::ConstructorOperations
};

#[derive(Debug, Clone)]
pub struct Config;

#[derive(Debug, Clone)]
pub struct Constructor;

impl ConstructorOperations for Constructor {

    type Artifact = Artifact;
    type Config = Config;

    fn default() -> Self::Artifact {

        Artifact::noop("curl".to_string()).with_checker(
            Checker::command_exists("curl".to_string())
        ) // Should already be installed on macOS

    }

    fn default_with_version(_ : &util::util::util::Version) -> Self::Artifact {
        Self::default()
    }

    fn from_config(_ : &util::util::util::Version, _ : &Self::Config) -> Self::Artifact {
        Self::default()
    }

}

#[cfg(test)]
pub mod test {

    use super::*;
    use util::movement_dir::MovementDir;

    #[cfg(target_os = "macos")]
    #[tokio::test]
    async fn test_curl_macos() -> Result<(), anyhow::Error> {
        
        let temp_home = tempfile::tempdir()?;   
    
        let dir = temp_home.path().to_path_buf();
        let movement_dir = MovementDir::new(&dir);
        let artifact = Constructor::default();

        artifact.install(&movement_dir).await?;

        let exists = match std::process::Command::new("curl").arg("--version").output() {
            Ok(output) => output.status.success(),
            Err(_) => false,
        };

        assert!(exists);

        Ok(())

    }


}