use util::{
    checker::Checker,
    artifact::Artifact,
    util::util::patterns::constructor::ConstructorOperations
};
use crate::known_artifacts::third_party::sys::curl;

#[derive(Debug, Clone)]
pub struct Config;

#[derive(Debug, Clone)]
pub struct Constructor;

impl ConstructorOperations for Constructor {

    type Artifact = Artifact;
    type Config = Config;

    fn default() -> Self::Artifact {

        #[cfg(target_os = "macos")]
        let avalanche = Artifact::self_contained_script(
            "brew".to_string(),
            r#"
            NONINTERACTIVE=1 /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/master/install.sh)"
            (echo; echo 'eval "$(/opt/homebrew/bin/brew shellenv)"') >> "$HOME/.zshrc"
            (echo; echo 'eval "$(/opt/homebrew/bin/brew shellenv)"') >> "$HOME/.bash_profile"
            (echo; echo 'export PATH="/opt/homebrew/bin:$PATH"') >> "$HOME/.zshrc"
            (echo; echo 'export PATH="/opt/homebrew/bin:$PATH"') >> "$HOME/.bash_profile"
            eval "$(/opt/homebrew/bin/brew shellenv)"
            brew --version
            "#.to_string(),
        ).with_dependencies(
            vec![
                curl::Constructor::default().into(),
            ].into_iter().collect()
        );

        #[cfg(not(target_os = "macos"))]
        let avalanche = Artifact::unsupported("brew".to_string());

        avalanche.with_checker(
            Checker::command_exists("brew".to_string())
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
    async fn test_brew() -> Result<(), anyhow::Error> {

        
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
        
        artifact.install(&movement_dir).await?;

        // add /usr/local/bin back to path
        test_helpers::clean_path(vec![
            "/usr/bin".to_string(), "/bin".to_string(), "/opt/homebrew/bin".to_string(), "/usr/local/bin".to_string()
        ])?;

        let exists = match std::process::Command::new("brew").arg("--version").output() {
            Ok(output) => output.status.success(),
            Err(_) => false,
        };

        assert!(exists);

        Ok(())

    }

    
}