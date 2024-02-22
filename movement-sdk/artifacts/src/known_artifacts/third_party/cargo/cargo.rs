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
        Artifact::self_contained_script(
            "cargo".to_string(),
            r#"
            curl https://sh.rustup.rs -sSf | sh -s -- -y
            source "$HOME/.cargo/env"
            cargo --version
            "#.to_string(),
        ).with_checker(
            Checker::command_exists("cargo".to_string())
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

    #[tokio::test]
    async fn test_cargo() -> Result<(), anyhow::Error> {
        
        let temp_home = tempfile::tempdir()?;
        let temp_cargo_bin = temp_home.path().join(".cargo/bin").to_str().unwrap().to_owned();

        // Add any other essential system paths. This example includes /usr/bin and /bin.
        test_helpers::clean_path(vec![
            "/usr/bin".to_string(), "/bin".to_string(), temp_cargo_bin
        ])?;
        std::env::set_var("HOME", temp_home.path());
        std::env::set_var("CARGO_HOME", temp_home.path().join(".cargo"));
        std::env::set_var("RUSTUP_HOME", temp_home.path().join(".rustup"));
    
        // Perform test actions here
        let dir = temp_home.path().to_path_buf();
        let movement_dir = MovementDir::new(&dir);
        let artifact = Constructor::default();

        artifact.install(&movement_dir).await?;

        let exists = match std::process::Command::new("cargo").arg("--version").output() {
            Ok(output) => output.status.success(),
            Err(_) => false,
        };

        assert!(exists);

        Ok(())

    }

}