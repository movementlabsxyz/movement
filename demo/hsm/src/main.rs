use clap::*;
use dotenv::dotenv;
use hsm_demo::cli;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
	// Load environment variables from .env file
	dotenv().ok();

	// run the CLI
	let hsm_demo = cli::HsmDemo::parse();
	hsm_demo.run().await?;
	Ok(())
}
