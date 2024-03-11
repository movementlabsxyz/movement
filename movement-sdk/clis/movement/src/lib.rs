// pub mod util;
pub mod manage;
pub mod ctl;
pub mod common;


use clap::*;
use manage::Manage;
use ctl::Ctl;

#[cfg(feature = "sui")]
use sui::sui_commands::SuiCommand;

#[cfg(feature = "aptos")]
use aptos::Tool;

use util::cli::Command;

const VERSION: &str = const_str::concat!(env!("CARGO_PKG_VERSION"));


#[derive(Parser)]
#[clap(rename_all = "kebab-case")]
pub enum MovementCommand {
    #[clap(subcommand)]
    Manage(Manage),
    #[clap(subcommand)]
    Ctl(Ctl),
    #[cfg(feature = "aptos")]
    #[clap(subcommand, about = "Run Aptos commands")]
    Aptos(Tool),
    #[cfg(feature = "sui")]
    #[clap(subcommand, about = "Run Sui commands")]
    Sui(SuiCommand)
}

#[async_trait::async_trait]
impl Command<String> for MovementCommand {

    async fn get_name(&self) -> String {
        "movement".to_string()
    }

    async fn execute(self) -> Result<String, anyhow::Error> {

      match self {
        MovementCommand::Manage(manage) => {
            manage.execute().await?;
            Ok("SUCCESS".to_string())
        },
        MovementCommand::Ctl(ctl) => {
            ctl.execute().await?;
            Ok("SUCCESS".to_string())
        },
        #[cfg(feature = "aptos")]
        MovementCommand::Aptos(aptos) => {
            aptos.execute().await.map_err(
               |e| anyhow::anyhow!("aptos error: {:?}", e)
            )?;
            Ok("SUCCESS".to_string())
        },
        #[cfg(feature = "sui")]
        MovementCommand::Sui(sui) => {
            sui.execute().await?;
            Ok("SUCCESS".to_string())
        },
        _ => {
            Ok("NOT FOUND".to_string())
        }
      }

    }

}

#[derive(Parser)]
#[clap(
    // name = env!("CARGO_BIN_NAME"),
    name = "movement",
    about = "A network of Move-based blockchains",
    rename_all = "kebab-case",
    author,
    version = VERSION,
    propagate_version = true,
)]
pub struct Movement {
    #[clap(subcommand)]
    pub command: MovementCommand
}