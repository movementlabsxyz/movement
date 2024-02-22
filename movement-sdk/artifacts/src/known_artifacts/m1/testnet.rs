use util::{
    artifact::Artifact,
    util::util::patterns::constructor::ConstructorOperations,
    util::util::version
};
use super::{
    testnet_vmid,
    testnet_cid,
    testnet_id,
    subnet,
    super::third_party::avalanche::{
            avalanche,
            avalanchego
        }
    
};

#[derive(Debug, Clone)]
pub struct Config;

#[derive(Debug, Clone)]
pub struct Constructor;

impl ConstructorOperations for Constructor {

    type Artifact = Artifact;
    type Config = Config;

    fn default() -> Self::Artifact {

        Self::default_with_version(&version::Version::Latest)

    }

    fn default_with_version(version : &util::util::util::Version) -> Self::Artifact {
        // source should have the same version
        let avalanche = avalanche::Constructor::default();
        let avalanchego = avalanchego::Constructor::default();
        let subnet = subnet::Constructor::default_with_version(version);
        let testnet_id = testnet_id::Constructor::default_with_version(version);
        let testnet_cid = testnet_cid::Constructor::default_with_version(version);
        let testnet_vmid = testnet_vmid::Constructor::default_with_version(version);

        Artifact::self_contained_script(
            "testnet".to_string(),
            r#"
            chmod -R 755 $MOVEMENT_DIR
            echo $MOVEMENT_DIR
            mkdir -p $MOVEMENT_DIR/avalanchego/plugins
            cp $MOVEMENT_DIR/bin/subnet $MOVEMENT_DIR/avalanchego/plugins/$(cat $MOVEMENT_DIR/rsc/testnet-vmid)
            "#.to_string(),
        ).with_dependencies(vec![
            avalanche.into(),
            avalanchego.into(),
            testnet_id.into(),
            testnet_cid.into(),
            testnet_vmid.into(),
            subnet.into(),
        ].into_iter().collect())

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
    async fn test_testnet() -> Result<(), anyhow::Error> {
        
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