use util::movement_dir::MovementDir;
use util::artifact::ArtifactDependency;
use util::movement_installer::{MovementInstaller, MovementInstallerOperations};
use crate::known_artifacts::registry;

/// The known artifact installer
pub async fn install(
    movement_dir : MovementDir,
    dependencies : Vec<ArtifactDependency>
) -> Result<MovementDir, anyhow::Error> {

    let movement_dir = movement_dir.sync()?;
    let registry = registry::Constructor::new().new_registry().await?;
    let movement_installer = MovementInstaller::new();

    let movement_dir = movement_installer.install(
        movement_dir,
        &registry,
        dependencies
    ).await?;
 
    Ok(movement_dir)

}

pub async fn get_movement_dir() -> Result<MovementDir, anyhow::Error> {

    let movement_dir = MovementDir::default();
    let movement_dir = movement_dir.sync()?;

    Ok(movement_dir)

}

/// The default known artifact installer
pub async fn install_default(
    dependencies : Vec<ArtifactDependency>
) -> Result<MovementDir, anyhow::Error> {

    let movement_dir = MovementDir::default();
    let movement_dir = movement_dir.sync()?;
    let registry = registry::Constructor::new().new_registry().await?;
    let movement_installer = MovementInstaller::new();

    let movement_dir = movement_installer.install(
        movement_dir,
        &registry,
        dependencies
    ).await?;
 
    Ok(movement_dir)

}