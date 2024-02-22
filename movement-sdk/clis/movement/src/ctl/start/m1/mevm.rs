use services::m1::mevm;
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
pub struct Mevm {
    
    #[clap(flatten)]
    pub version_args : VersionArgs,

    #[clap(flatten)]
    pub installation_args : InstallationArgs,

    #[clap(flatten)]
    pub config_args : ConfigArgs,

}

#[derive(Debug, Parser, Clone)]
pub struct ConfigArgs {

    #[clap(
        long,
        default_value = mevm::Config::DEFAULT_EVM_SENDER
    )]
    pub evm_sender : String,

    #[clap(
        long,
        default_value = mevm::Config::DEFAULT_FAUCET_SENDER
    )]
    pub faucet_sender : String,

    #[clap(
        long,
        default_value = mevm::Config::DEFAULT_NODE_URL
    )]
    pub node_url : String,

}

impl From <mevm::Config> for ConfigArgs {
    fn from(config : mevm::Config) -> ConfigArgs {
        ConfigArgs {
            evm_sender : config.evm_sender,
            faucet_sender : config.faucet_sender,
            node_url : config.node_url
        }
    }
}

impl From<ConfigArgs> for mevm::Config {
    fn from(config_args : ConfigArgs) -> mevm::Config {
        mevm::Config {
            evm_sender : config_args.evm_sender,
            faucet_sender : config_args.faucet_sender,
            node_url : config_args.node_url
        }
    }
}


#[async_trait]
impl Command<String> for Mevm {

    async fn get_name(&self) -> String {
        "mevm".to_string()
    }

    async fn execute(self) -> Result<String, anyhow::Error> {

        let movement_dir = MovementDir::default();

        // todo: handle config and version
        let config : mevm::Config = self.config_args.clone().into();
        let version : Version = self.version_args.try_into()?;

        let service = mevm::Constructor::from_config(
            &version,
            &config
        );

        service.start(&movement_dir).await?;

        Ok("SUCCESS".to_string())
    }

}