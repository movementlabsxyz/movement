use clap::Subcommand;
use util::cli::Command;
use super::{
    localnet::Localnet,
    testnet::Testnet,
    mevm::Mevm,
    proxy::Proxy
};

#[derive(Subcommand, Debug)]
#[clap(
    rename_all = "kebab-case",
    about = "Start an M1 service"
)]
pub enum M1 {
    Localnet(Localnet),
    Testnet(Testnet),
    Mevm(Mevm),
    Proxy(Proxy)
}

#[async_trait::async_trait]
impl Command<String> for M1 {

    async fn get_name(&self) -> String {
        "start".to_string()
    }

    async fn execute(self) -> Result<String, anyhow::Error> {

        match self {
            M1::Localnet(localnet) => localnet.execute().await?,
            M1::Testnet(testnet) => testnet.execute().await?,
            M1::Mevm(mevm) => mevm.execute().await?,
            M1::Proxy(proxy) => proxy.execute().await?
        };

        Ok("SUCCESS".to_string())
    }

}