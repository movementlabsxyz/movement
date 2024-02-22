use util::{
    release::Release,
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

       Artifact::resource_release(
            "testnet-vmid".to_string(),
            Release::github_release(
                "movemntdev".to_string(),
                "M1".to_string(),
                "testnet-vmid".to_string(),
                "".to_string()
            )
        )

    }

    fn default_with_version(version : &util::util::util::Version) -> Self::Artifact {
        Self::default().with_version(version.clone())
    }

    fn from_config(version : &util::util::util::Version, _ : &Self::Config) -> Self::Artifact {
        Self::default_with_version(version)
    }

}

#[cfg(test)]
pub mod test {

    use super::*;
    use util::movement_dir::MovementDir;

    #[tokio::test]
    async fn test_testnet_vmid_with_submodules() -> Result<(), anyhow::Error> {
        
        let temp_home = tempfile::tempdir()?;

        // Add any other essential system paths. This example includes /usr/bin and /bin.
        test_helpers::clean_path(vec![
            "/usr/bin".to_string(), "/bin".to_string(),
        ])?;
        std::env::set_var("HOME", temp_home.path());
    
        // Perform test actions here
        let dir = temp_home.path().to_path_buf();
        let movement_dir = MovementDir::new(&dir);
        let artifact = Constructor::default();

        test_helpers::clean_path(vec![
            "/usr/bin".to_string(), "/bin".to_string(),
        ])?;
        artifact.install(&movement_dir).await?;
        assert!(tokio::fs::try_exists(
            dir.join("rsc").join("testnet-vmid")
        ).await?);

        Ok(())

    }



}