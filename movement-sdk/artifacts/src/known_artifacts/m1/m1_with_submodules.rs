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

       Artifact::source_tar_gz_release(
            "m1-with-submodules".to_string(),
            Release::github_release(
                "movemntdev".to_string(),
                "M1".to_string(),
                "m1-with-submodules".to_string(),
                ".tar.gz".to_string()
            )
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
    async fn test_m1_with_submodules() -> Result<(), anyhow::Error> {
        
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


        // ls movement_dir
        let mut entries = tokio::fs::read_dir(&movement_dir.path.join("src/m1-with-submodules")).await?;
        while let Some(entry) = entries.next_entry().await? {
            println!("{:?}", entry.path());
        }

        assert!(tokio::fs::try_exists(
            dir.join("src").join("m1-with-submodules")
        ).await?);

        Ok(())

    }


}