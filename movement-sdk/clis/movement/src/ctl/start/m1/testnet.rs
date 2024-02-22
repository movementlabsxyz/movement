use services::m1::testnet;
use async_trait::async_trait;
use clap::Parser;
use util::{cli::Command, util::util::constructor::ConstructorOperations};
use crate::manage::{
    InstallationArgs,
    VersionArgs
};
use util::util::util::Version;
use util::service::ServiceOperations;
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

        // todo: handle config and version
        let config : testnet::Config = self.clone().into();
        let version : Version = self.version_args.try_into()?;

        let service = testnet::Constructor::default();

        service.start(&movement_dir).await?;

        Ok("SUCCESS".to_string())
    }

}