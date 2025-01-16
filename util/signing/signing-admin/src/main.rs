use clap::{Parser, Subcommand};
use anyhow::Result;
use signing_admin::{aws::AwsKey, key_manager::KeyManager, vault::VaultKey, notify::notify_application};

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
        // Select the appropriate key manager based on the backend
        let key_manager: Box<dyn KeyManager<PublicKey = Vec<u8>>> = match backend.as_str() {
                "aws" => Box::new(AwsKey::new()),
                "vault" => Box::new(VaultKey::new()),
                _ => return Err(anyhow::anyhow!("Unsupported backend: {}", backend)),
        };

        // Rotate the key
        let new_key_id = key_manager.rotate_key(&canonical_string)?;
        println!("Key rotated. New Key ID: {}", new_key_id);

        // Fetch the new public key
        let new_public_key = key_manager.fetch_public_key(&new_key_id)?;
        println!("Retrieved public key: {:?}", new_public_key);

        // Notify the application
        notify_application(&application_url, &new_public_key).await?;
        println!("Application updated with new public key: {:?}", new_public_key);

        Ok(())
}
