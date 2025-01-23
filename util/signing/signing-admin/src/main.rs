use anyhow::{Context, Result};
use clap::Parser;
use signing_admin::{application::HttpApplication, backend::{aws::AwsBackend, vault::VaultBackend}};

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
                cli::Commands::RotateKeyRecover {} => {
                        // Load WAL file to determine the backend
                        let wal = cli::rotate_key::load_wal_from_file()
                                .context("Failed to load WAL file for recovery")?;
                        
                        match wal.backend_name.as_str() {
                                "vault" => {
                                        cli::rotate_key::recover::<HttpApplication, VaultBackend>().await?;
                                }
                                "aws" => {
                                        cli::rotate_key::recover::<HttpApplication, AwsBackend>().await?;
                                }
                                _ => {
                                        return Err(anyhow::anyhow!("Unsupported backend: {}", wal.backend_name));
                                }
                        }
                }
        }

        Ok(())
}
