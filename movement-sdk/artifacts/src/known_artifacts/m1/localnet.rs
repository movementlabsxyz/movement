use util::{
    artifact::Artifact,
    util::util::patterns::constructor::ConstructorOperations
};
use super::m1_with_submodules;

#[derive(Debug, Clone)]
pub struct Config;

#[derive(Debug, Clone)]
pub struct Constructor;

impl ConstructorOperations for Constructor {

    type Artifact = Artifact;
    type Config = Config;

    fn default() -> Self::Artifact {

        Artifact::noop("localnet".to_string())
        .with_dependencies(vec![
            m1_with_submodules::Constructor::default().into()
        ].into_iter().collect()) // Should already be installed on macOS

    }

    fn default_with_version(version : &util::util::util::Version) -> Self::Artifact {
        // source should have the same version
        let source = m1_with_submodules::Constructor::default_with_version(version);
        Artifact::noop("localnet".to_string())
        .with_dependencies(vec![
            source.into()
        ].into_iter().collect())
    }

    fn from_config(version : &util::util::util::Version, _ : &Self::Config) -> Self::Artifact {
        Self::default_with_version(version)
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