use util::{
    checker::Checker,
    artifact::Artifact,
    util::util::patterns::constructor::ConstructorOperations
};
use crate::known_artifacts::third_party::sys::git;

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

            echo $MOVEMENT_DIR
            mkdir -p $MOVEMENT_DIR/workspace
            cd $MOVEMENT_DIR/workspace
            mkdir -p $MOVEMENT_DIR/bin
            git clone https://github.com/ava-labs/avalanchego
            cd avalanchego
            git checkout tags/v1.10.12
            ./scripts/build.sh

            mkdir -p $HOME/bin
            mv ./build/avalanchego $HOME/bin/avalanchego
            cp $HOME/bin/avalanchego $MOVEMENT_DIR/bin/avalanchego
            cd ..
            rm -rf $MOVEMENT_DIR/workspace/avalanchego

            (echo; echo 'export PATH="$HOME/bin:$PATH"') >> "$HOME/.zshrc"
            (echo; echo 'export PATH="$HOME/bin:$PATH"') >> "$HOME/.bash_profile"
            export PATH="$HOME/bin:$PATH"

            avalanchego --version
            "#.to_string(),
        ).with_dependencies(
            vec![
                git::Constructor::default().into(),
            ].into_iter().collect()
        ).with_checker(
            Checker::Noop
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
    async fn test_avalanchego() -> Result<(), anyhow::Error> {
        
        let temp_home = tempfile::tempdir()?;
        std::env::set_var("HOME", temp_home.path());
    
        // Perform test actions here
        let dir = temp_home.path().to_path_buf();
        let movement_dir = MovementDir::new(&dir);
        let artifact = Constructor::default();

        artifact.install(&movement_dir).await?;

        Ok(())

    }

}