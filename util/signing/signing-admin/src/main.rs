use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use signing_admin::{aws::AwsKey, key_manager::KeyManager, notify::notify_application, vault::VaultKey};

#[derive(Parser, Debug)]
#[clap(name = "signing-admin", about = "CLI for managing signing keys")]
struct CLI {
        #[clap(subcommand)]
        command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
        /// Rotate a signing key
        RotateKey {
                #[clap(long, help = "Canonical string of the key (alias for the backend key)")]
                canonical_string: String,

                #[clap(long, help = "Application URL to notify about the key rotation")]
                application_url: String,

                #[clap(long, help = "Backend to use (e.g., 'vault', 'aws')")]
                backend: String,
        },
}

#[tokio::main]
async fn main() -> Result<()> {
        let cli = CLI::parse();

        match cli.command {
                Commands::RotateKey {
                        canonical_string,
                        application_url,
                        backend,
                } => {
                        rotate_key(canonical_string, application_url, backend).await?;
                }
        }

        Ok(())
}

async fn rotate_key(
        canonical_string: String,
        application_url: String,
        backend: String,
) -> Result<()> {
        let key_manager: Box<dyn KeyManager<PublicKey = Vec<u8>>> = match backend.as_str() {
                "vault" => Box::new(VaultKey::new()),
                "aws" => {
                        Box::new(AwsKey::new())
                },
                _ => return Err(anyhow::anyhow!("Unsupported backend: {}", backend)),
        };

        let new_key_id = key_manager.rotate_key(&canonical_string).await?;
        println!("Key rotated. New Key ID: {}", new_key_id);

        let new_public_key = key_manager.fetch_public_key(&new_key_id).await?;
        println!("Retrieved public key: {:?}", new_public_key);

        notify_application(&application_url, &new_public_key).await?;
        println!("Application updated with new public key: {:?}", new_public_key);

        Ok(())
}
