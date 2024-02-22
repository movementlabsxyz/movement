use clap::Subcommand;
use util::cli::Command;
use super::m1::M1;

#[derive(Subcommand, Debug)]
#[clap(
    rename_all = "kebab-case",
    about = "Start a Movement service"
)]
pub enum Start {
    #[clap(subcommand)]
    M1(M1)
}

#[async_trait::async_trait]
impl Command<String> for Start {

    async fn get_name(&self) -> String {
        "start".to_string()
    }

    async fn execute(self) -> Result<String, anyhow::Error> {

        match self {
            Start::M1(m1) => m1.execute().await?
        };

        Ok("SUCCESS".to_string())
    }

}