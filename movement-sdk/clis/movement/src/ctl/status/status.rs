use async_trait::async_trait;
use clap::Subcommand;

#[derive(Subcommand, Debug)]
#[clap(
    rename_all = "kebab-case",
    about = "Get the status of a Movement service"
)]
pub enum Status {
    
}