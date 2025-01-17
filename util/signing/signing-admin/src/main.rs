use anyhow::Result;
use clap::Parser;

mod cli;

#[tokio::main]
async fn main() -> Result<()> {
        let cli = cli::CLI::parse();

        match cli.command {
                cli::Commands::RotateKey {
                        canonical_string,
                        application_url,
                        backend,
                } => {
                        cli::rotate_key::rotate_key(canonical_string, application_url, backend).await?;
                }
        }

        Ok(())
}
