use util::{
    checker::Checker,
    artifact::Artifact,
    util::util::patterns::constructor::ConstructorOperations,
};

#[derive(Debug, Clone)]
pub struct Config;

#[derive(Debug, Clone)]
pub struct Constructor;

impl ConstructorOperations for Constructor {
    type Artifact = Artifact;
    type Config = Config;

    fn default() -> Self::Artifact {
        #[cfg(target_os = "macos")]
        let node_artifact = Artifact::self_contained_script(
            "node".to_string(),
            r#"
            brew install node@18
            brew link --force --overwrite node@18
        
            # Verify installation
            node --version
            "#
            .to_string(),
        );
    
        #[cfg(target_os = "linux")]
        let node_artifact = Artifact::self_contained_script(
            "node".to_string(),
            r#"
            sudo apt-get update
    
            curl -fsSL https://deb.nodesource.com/setup_18.x | sudo -E bash -
            sudo apt-get install -y nodejs
    
            # Verify installation
            node --version
            "#
            .to_string(),
        );
    
        // Placeholder for Windows, adjust as necessary for your application
        #[cfg(target_os = "windows")]
        let node_artifact = Artifact::unsupported("node".to_string());
    
        node_artifact.with_checker(
            Checker::command_exists("node".to_string())
        )
        
    }
    

    fn default_with_version(_: &util::util::util::Version) -> Self::Artifact {
        Self::default()
    }

    fn from_config(_: &util::util::util::Version, _: &Self::Config) -> Self::Artifact {
        Self::default()
    }
}

#[cfg(test)]
pub mod test {
    use super::*;
    use util::movement_dir::MovementDir;

    #[tokio::test]
    async fn test_node_installation() -> Result<(), anyhow::Error> {
        let temp_home = tempfile::tempdir()?;
        let dir = temp_home.path().to_path_buf();
        let movement_dir = MovementDir::new(&dir);
        let artifact = Constructor::default();

        test_helpers::clean_path(vec![
            "/usr/bin".to_string(), "/bin".to_string(),
            "/opt/homebrew/bin".to_string(), "/usr/local/bin".to_string(),
        ])?;
        std::env::set_var("HOME", temp_home.path());

        artifact.install(&movement_dir).await?;

        let exists = match std::process::Command::new("node").arg("--version").output() {
            Ok(output) => output.status.success(),
            Err(_) => false,
        };

        assert!(exists);

        Ok(())
    }
}
