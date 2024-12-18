mod cli;
mod hsm;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Service};
use hsm::{aws::AwsKms, google::GoogleKms, vault::HashiCorpVault};
use dotenv::dotenv;
use hsm_demo::{action_stream, Application};

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok(); // Load environment variables from .env file
    let cli = Cli::parse();

    // Select the HSM implementation based on CLI input
    let hsm = match cli.service {
        Service::Aws(args) => {
            println!("Using AWS KMS with {:?} key", args.key_type);
            AwsKms::try_from_env()
                .await?
                .create_key()
                .await?
                .fill_with_public_key()
                .await?
        }
        Service::Gcp(args) => {
            println!("Using Google Cloud KMS with {:?} key", args.key_type);
            GoogleKms::try_from_env()
                .await?
                .create_key_ring()
                .await?
                .create_key()
                .await?
                .fill_with_public_key()
                .await?
        }
        Service::Vault(args) => {
            println!("Using HashiCorp Vault with {:?} key", args.key_type);
            HashiCorpVault::try_from_env()
                .and_then(|vault| vault.create_key())
                .await?
                .fill_with_public_key()
                .await?
        }
    };

    // Initialize the streams
    let random_stream = action_stream::random::Random;
    let notify_verify_stream = action_stream::notify_verify::NotifyVerify::new();
    let join_stream = action_stream::join::Join::new(vec![
        Box::new(random_stream),
        Box::new(notify_verify_stream),
    ]);

    // Run the application
    let mut app = Application::new(Box::new(hsm), Box::new(join_stream));
    app.run().await?;

    Ok(())
}
