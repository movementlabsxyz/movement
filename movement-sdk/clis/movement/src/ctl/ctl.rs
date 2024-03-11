use clap::Subcommand;
use super::{
    start::Start,
    status::Status,
    stop::Stop,
};
use util::cli::Command;

#[derive(Subcommand, Debug)]
#[clap(
    rename_all = "kebab-case",
    about = "Control Movement services"
)]
pub enum Ctl {
    #[clap(subcommand)]
    Start(Start),
    #[clap(subcommand)]
    Status(Status),
    #[clap(subcommand)]
    Stop(Stop),
}

#[async_trait::async_trait]
impl Command<String> for Ctl {

    async fn get_name(&self) -> String {
        "ctl".to_string()
    }

    async fn execute(self) -> Result<String, anyhow::Error> {

        match self {
            Ctl::Start(start) => start.execute().await?,
            _ => unimplemented!()
        };

        Ok("SUCCESS".to_string())
    }

}

