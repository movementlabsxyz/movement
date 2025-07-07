use clap::Parser;

mod exec;
mod local;

#[derive(Parser, Debug)]
pub struct Replicat;

impl Replicat {
	pub async fn execute(&self) -> Result<(), anyhow::Error> {
		exec::exec().await
	}
}
