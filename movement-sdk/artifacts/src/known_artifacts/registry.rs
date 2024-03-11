use util::artifact::registry::{
    ArtifactRegistry,
    ArtifactRegistryOperations
};
use crate::known_artifacts::*;
use util::util::util::patterns::constructor::ConstructorOperations;


#[derive(Debug, Clone)]
pub struct Constructor;

impl Constructor {

    pub fn new() -> Self {
        Self {}
    }

    pub async fn new_registry(&self) -> Result<ArtifactRegistry, anyhow::Error> {

        let registry = ArtifactRegistry::in_memory();

        // ! we don't actually have to register anything currently 
        // ! because all deps are fully specified artifacts
        // ! but the below is an example if you ever had a need to register something
        // ! because you don't want to fully specify a dependency
        // m1 
        let subnet_latest = m1::subnet::Constructor::default();
        registry.register(&subnet_latest).await?;

        Ok(registry)

    }

}

#[cfg(test)]
pub mod test {

    use super::*;
    use util::movement_installer::{
        MovementInstaller,
        MovementInstallerOperations
    };
    use util::movement_dir::MovementDir;

    #[tokio::test]
    pub async fn test_install_with_registry() -> Result<(), anyhow::Error> {

        let temp_dir = tempfile::tempdir()?;
        let path = temp_dir.path().to_path_buf();
        let movement_dir = MovementDir::new(&path);
        movement_dir.store()?;
        let movement_installer = MovementInstaller::new();
        let registry = Constructor::new().new_registry().await?;

        movement_installer.install(
            movement_dir,
            &registry,
            vec![
                m1::subnet::Constructor::default().into()
            ]
        ).await?;

        Ok(())

    }

}