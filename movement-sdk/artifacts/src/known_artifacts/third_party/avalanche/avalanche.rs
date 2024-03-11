use util::{
    checker::Checker,
    artifact::Artifact,
    util::util::patterns::constructor::ConstructorOperations
};
use crate::known_artifacts::third_party::sys::{curl, brew};

#[derive(Debug, Clone)]
pub struct Config;

#[derive(Debug, Clone)]
pub struct Constructor;

impl ConstructorOperations for Constructor {

    type Artifact = Artifact;
    type Config = Config;

    fn default() -> Self::Artifact {

        #[cfg(not(target_os = "windows"))]
        let avalanche = Artifact::self_contained_script(
            "avalanche".to_string(),
            r#"
            echo $MOVEMENT_DIR
            curl -sSfL https://raw.githubusercontent.com/ava-labs/avalanche-cli/main/scripts/install.sh | sh -s

            # add $HOME/bin to path
            (echo; echo 'export PATH="$HOME/bin:$PATH"') >> "$HOME/.zshrc"
            (echo; echo 'export PATH="$HOME/bin:$PATH"') >> "$HOME/.bash_profile"
            export PATH="$HOME/bin:$PATH"

            # copy avalanche to movement bin
            mkdir -p $MOVEMENT_DIR/bin
            cp $(which avalanche) $MOVEMENT_DIR/bin/avalanche

            avalanche --version
            "#.to_string(),
        ).with_dependencies(
            vec![
                curl::Constructor::default().into(),
                #[cfg(target_os = "macos")]
                brew::Constructor::default().into(),
            ].into_iter().collect()
        );

        #[cfg(target_os = "windows")]
        let avalanche = Artifact::unsupported("avalanche".to_string());

        avalanche.with_checker(
            Checker::command_exists("avalanche".to_string())
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
    async fn test_avalanche() -> Result<(), anyhow::Error> {
        
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

        // explicit deps installation
        curl::Constructor::default().install(&movement_dir).await?;
        brew::Constructor::default().install(&movement_dir).await?;

        test_helpers::clean_path(vec![
            "/usr/bin".to_string(), "/bin".to_string(), "/opt/homebrew/bin".to_string()
        ])?;
        artifact.install(&movement_dir).await?;

        // add /usr/local/bin back to path
        test_helpers::clean_path(vec![
            "/usr/bin".to_string(), "/bin".to_string(), "/usr/local/bin".to_string(), "/opt/homebrew/bin".to_string()
        ])?;

        let exists = match std::process::Command::new("avalanche").arg("--version").output() {
            Ok(output) => output.status.success(),
            Err(_) => false,
        };

        assert!(exists);

        Ok(())

    }

    #[tokio::test]
    async fn test_fake_should_not_work() -> Result<(), anyhow::Error> {
        
        let temp_home = tempfile::tempdir()?;
        let temp_avalanche_bin = temp_home.path().join(".avalanche/bin").to_str().unwrap().to_owned();

        // 
        test_helpers::clean_path(vec![
            "/usr/bin".to_string(), "/bin".to_string(), temp_avalanche_bin
        ])?;
        std::env::set_var("HOME", temp_home.path());
        std::env::set_var("avalanche_HOME", temp_home.path().join(".avalanche"));
        std::env::set_var("RUSTUP_HOME", temp_home.path().join(".rustup"));

    
        // Perform test actions here
        let dir = temp_home.path().to_path_buf();
        let movement_dir = MovementDir::new(&dir);
        let artifact = Fake::default();

        artifact.install(&movement_dir).await?;

        let exists = match std::process::Command::new("avalanche").arg("--version").output() {
            Ok(output) => output.status.success(),
            Err(_) => false,
        };

        assert!(!exists);

        Ok(())

    }

    #[derive(Debug, Clone)]
    pub struct Fake;

    impl ConstructorOperations for Fake {

        type Artifact = Artifact;
        type Config = Config;

        fn default() -> Self::Artifact {
            Artifact::self_contained_script(
                "avalanche".to_string(),
                r#"
                    echo fake
                "#.to_string(),
            )
        }

        fn default_with_version(_ : &util::util::util::Version) -> Self::Artifact {
            Self::default()
        }

        fn from_config(_ : &util::util::util::Version, _ : &Self::Config) -> Self::Artifact {
            Self::default()
        }

    }


}