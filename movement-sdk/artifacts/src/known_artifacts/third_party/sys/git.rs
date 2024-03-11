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

        #[cfg(target_os = "macos")]
        let git = Artifact::noop("git".to_string()); // Should already be installed on macOS

        #[cfg(not(target_os = "macos"))]
        let git = Artifact::self_contained_script(
            "git".to_string(),
            r#"
            sudo apt-get update
            sudo apt-get install git -y
            git --version
            "#.to_string(),
        );

        // todo: update for windows

        git.with_checker(
            Checker::command_exists("git".to_string())
        )

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
    async fn test_git_macos() -> Result<(), anyhow::Error> {
        
        let temp_home = tempfile::tempdir()?;   
    
        let dir = temp_home.path().to_path_buf();
        let movement_dir = MovementDir::new(&dir);
        let artifact = Constructor::default();

        artifact.install(&movement_dir).await?;

        let exists = match std::process::Command::new("git").arg("--version").output() {
            Ok(output) => output.status.success(),
            Err(_) => false,
        };

        assert!(exists);

        Ok(())

    }

    #[cfg(not(target_os = "macos"))]
    #[tokio::test]
    async fn test_fake_should_not_work() -> Result<(), anyhow::Error> {
        
        let temp_home = tempfile::tempdir()?;

        let system_paths = vec!["/usr/bin", "/bin"];
        let new_path = system_paths.into_iter().map(String::from)
            .collect::<Vec<_>>()
            .join(":");
    
        // Override environment variables
        std::env::set_var("HOME", temp_home.path());
        std::env::set_var("CARGO_HOME", temp_home.path().join(".cargo"));
        std::env::set_var("RUSTUP_HOME", temp_home.path().join(".rustup"));
        std::env::set_var("PATH", new_path);

    
        // Perform test actions here
        let dir = temp_home.path().to_path_buf();
        let movement_dir = MovementDir::new(&dir);
        let artifact = Fake::default();

        artifact.install(&movement_dir).await?;

        let exists = match std::process::Command::new("git").arg("--version").output() {
            Ok(output) => output.status.success(),
            Err(_) => false,
        };

        assert!(!exists);

        Ok(())

    }

}