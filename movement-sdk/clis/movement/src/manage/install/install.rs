use clap::{Subcommand, Parser};
use super::m1;
use util::cli::Command;

#[derive(Debug, Parser)]
pub struct All;

#[derive(Subcommand, Debug)]
#[clap(
    rename_all = "kebab-case",
    about = "Install Movement artifacts"
)]
pub enum Install {
    All(All),
    #[clap(subcommand)]
    M1(m1::M1)
}

#[async_trait::async_trait]
impl Command<String> for Install {

    async fn get_name(&self) -> String {
        "install".to_string()
    }

    async fn execute(self) -> Result<String, anyhow::Error> {

        match self {
            Install::M1(m1) => {
                m1.execute().await?;
            },
            Install::All(_) => {
                
            }
        }

        Ok("SUCCESS".to_string())
    }

}
