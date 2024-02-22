use async_trait::async_trait;
use clap::Parser;
use util::{cli::Command, util::util::constructor::ConstructorOperations};
use artifacts::known_artifacts::{
    install,
    m1::m1_with_submodules
};
use crate::manage::{
    InstallationArgs,
    VersionArgs
};
use util::util::util::Version;
use util::movement_dir::MovementDir;


#[derive(Debug, Parser, Clone)]
pub struct Localnet {
    
    #[clap(flatten)]
    pub version_args : VersionArgs,

    #[clap(flatten)]
    pub installation_args : InstallationArgs


}

impl Into<m1_with_submodules::Config> for Localnet {
    fn into(self) -> m1_with_submodules::Config {
        m1_with_submodules::Config
    }
}

#[async_trait]
impl Command<String> for Localnet {

    async fn get_name(&self) -> String {
        "localnet".to_string()
    }

    async fn execute(self) -> Result<String, anyhow::Error> {

        let movement_dir = MovementDir::default();
        let config : m1_with_submodules::Config = self.clone().into();
        let version : Version = self.version_args.try_into()?;

        let artifact = m1_with_submodules::Constructor::from_config(
            &version,
            &config
        );

        install(
            movement_dir,
            vec![artifact.into()]
        ).await?;

        Ok("SUCCESS".to_string())
    }

}