use async_trait::async_trait;
use clap::Parser;
use util::{cli::Command, util::util::constructor::ConstructorOperations};
use artifacts::known_artifacts::{
    m1::m1_with_submodules,
    m1::testnet,
   install
};
use crate::manage::{
    InstallationArgs,
    VersionArgs
};
use util::util::util::Version;
use util::movement_dir::MovementDir;

#[derive(Debug, Parser, Clone)]
pub struct Testnet {
    
    #[clap(flatten)]
    pub version_args : VersionArgs,

    #[clap(flatten)]
    pub installation_args : InstallationArgs

}

impl Into<testnet::Config> for Testnet {
    fn into(self) -> testnet::Config {
        testnet::Config
    }
}


#[async_trait]
impl Command<String> for Testnet {

    async fn get_name(&self) -> String {
        "testnet".to_string()
    }

    async fn execute(self) -> Result<String, anyhow::Error> {

        let movement_dir = MovementDir::default();
        let config : testnet::Config = self.clone().into();
        let version : Version = self.version_args.try_into()?;

        let testnet = testnet::Constructor::from_config(
            &version,
            &config
        );

        let source = m1_with_submodules::Constructor::from_config(
            &version,
            &m1_with_submodules::Config
        );

        install(
            movement_dir,
            vec![
                testnet.into(),
                source.into()
            ]
        ).await?;

        Ok("SUCCESS".to_string())
    }

}