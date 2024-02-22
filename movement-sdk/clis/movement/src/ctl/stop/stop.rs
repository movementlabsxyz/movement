use async_trait::async_trait;
use clap::Subcommand;

#[derive(Subcommand, Debug)]
#[clap(
    rename_all = "kebab-case",
    about = "Stop a Movement service"
)]
pub enum Stop {
    
}