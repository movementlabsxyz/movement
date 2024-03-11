use services::m1::proxy;
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
pub struct Proxy {
    
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
        default_value = proxy::Config::BASE_URL
    )]
    pub base_url : String,

    #[clap(
        long,
        default_value = proxy::Config::SUBNET_ID
    )]
    pub subnet_id : String,

}

impl Default for ConfigArgs {
    fn default() -> Self {
        proxy::Config::default().into()
    }
}

impl From<proxy::Config> for ConfigArgs {
    fn from(config : proxy::Config) -> ConfigArgs {
        ConfigArgs {
            base_url : config.base_url,
            subnet_id : config.subnet_id
        }
    }
}

impl From<ConfigArgs> for proxy::Config {
    fn from(config_args : ConfigArgs) -> proxy::Config {
        proxy::Config {
            base_url : config_args.base_url,
            subnet_id : config_args.subnet_id
        }
    }
}


#[async_trait]
impl Command<String> for Proxy {

    async fn get_name(&self) -> String {
        "proxy".to_string()
    }

    async fn execute(self) -> Result<String, anyhow::Error> {

        let movement_dir = MovementDir::default();

        // todo: handle config and version
        let config : proxy::Config = self.config_args.clone().into();
        let version : Version = self.version_args.try_into()?;

        let service = proxy::Constructor::from_config(
            &version,
            &config
        );

        service.start(&movement_dir).await?;

        Ok("SUCCESS".to_string())
    }

}