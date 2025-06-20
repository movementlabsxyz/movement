pub mod exec;
pub mod local;

use clap::Parser;

#[derive(Parser, Debug)]
pub struct Da;

impl Da {
	pub async fn execute(&self) -> Result<(), anyhow::Error> {
		exec::exec().await
	}
}
