use async_trait::async_trait;
use clap::{Subcommand, Parser};
use util::cli::Command;
use super::{
    localnet,
    testnet
};

#[derive(Debug, Parser)]
pub struct All;

#[derive(Subcommand, Debug)]
#[clap(
    rename_all = "kebab-case",
    about = "Install M1 artifacts"
)]
pub enum M1 {
    All(All),
    Localnet(localnet::Localnet),
    Testnet(testnet::Testnet)
}

#[async_trait]
impl Command<String> for M1 {

    async fn get_name(&self) -> String {
        "m1".to_string()
    }

    async fn execute(self) -> Result<String, anyhow::Error> {

        match self {
            M1::Localnet(localnet) => {
                localnet.execute().await?;
            },
            M1::Testnet(testnet) => {
                testnet.execute().await?;
            },
            M1::All(_) => {
                
            }
        }

        Ok("SUCCESS".to_string())
    }

}