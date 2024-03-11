use util::{
    release::Release,
    artifact::Artifact,
    util::util::patterns::constructor::ConstructorOperations
};
use crate::known_artifacts::{
    third_party::cargo,
    m1::m1_with_submodules
};

#[derive(Debug, Clone)]
pub struct Config {
    pub build : bool,
}

#[derive(Debug, Clone)]
pub struct Constructor;

impl Constructor {

    pub fn download() -> Artifact {
        Artifact::bin_release(
            "subnet".to_string(),
            Release::github_platform_release(
                "movemntdev".to_string(),
                "m1".to_string(),
                "subnet".to_string(),
                "".to_string()
            )
        )
    }

    pub fn build() -> Artifact {

        Artifact::self_contained_script(
            "subnet".to_string(),
            r#"
            mkdir -p $MOVEMENT_DIR/bin
            source "$HOME/.cargo/env"
            echo $MOVEMENT_DIR
            cd $MOVEMENT_DIR/src/m1-with-submodules/m1
            cargo build -p subnet
            # for now use the debug build
            cp target/debug/subnet $MOVEMENT_DIR/bin/subnet
            "#.to_string(),
        ).with_dependencies(vec![
            cargo::Constructor::default().into(),
            m1_with_submodules::Constructor::default().into()
        ].into_iter().collect())

    }
}

impl ConstructorOperations for Constructor {

    type Artifact = Artifact;
    type Config = Config;

    fn default() -> Self::Artifact {

        #[cfg(target_os = "linux")]
        let movement = Self::download();

        #[cfg(target_os = "macos")]
        let movement = Self::build();

        #[cfg(target_os = "windows")]
        let movement = Artifact::unsupported("subnet".to_string());

        movement

    }

    fn default_with_version(version : &util::util::util::Version) -> Self::Artifact {
        Self::default()
        .with_version(version.clone())
    }

    fn from_config(_ : &util::util::util::Version, config : &Self::Config) -> Self::Artifact {

        if config.build {
            Self::build()
        } else {
            Self::default()
        }

    }

}

#[cfg(test)]
pub mod test {

    use super::*;
    use util::movement_dir::MovementDir;

    #[cfg(target_os = "macos")]
    #[tokio::test]
    async fn test_movement() -> Result<(), anyhow::Error> {
        
        let temp_home = tempfile::tempdir()?;

        // Add any other essential system paths. This example includes /usr/bin and /bin.
        test_helpers::clean_path(vec![
            "/usr/bin".to_string(), 
            "/bin".to_string(),
            temp_home.path().join("bin").to_str().unwrap().to_string(),
            temp_home.path().join(".cargo/env").to_str().unwrap().to_string(),
        ])?;
        std::env::set_var("HOME", temp_home.path());
        std::env::set_var("CARGO_HOME", temp_home.path().join(".cargo"));
        std::env::set_var("RUSTUP_HOME", temp_home.path().join(".rustup"));
    
        // Perform test actions here
        let dir = temp_home.path().to_path_buf();
        let movement_dir = MovementDir::new(&dir);
        let artifact = Constructor::default();

        cargo::Constructor::default().install(&movement_dir).await?;
        m1_with_submodules::Constructor::default().install(&movement_dir).await?;
        artifact.install(&movement_dir).await?;

        // add /usr/local/bin back to path
        /*test_helpers::clean_path(vec![
            "/usr/bin".to_string(), "/bin".to_string(), "/usr/local/bin".to_string(), 
            dir.join("bin").to_str().unwrap().to_string()
        ])?;*/

        let exists = match std::process::Command::new("movement").arg("--version").output() {
            Ok(output) => output.status.success(),
            Err(_) => false,
        };

        assert!(exists);

        Ok(())

    }


}