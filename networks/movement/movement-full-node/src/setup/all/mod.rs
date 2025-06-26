use super::da;
use super::full_node;
use clap::Parser;

#[derive(Parser, Debug)]
pub struct All;

impl All {
	pub async fn execute(&self) -> Result<(), anyhow::Error> {
		full_node::exec::exec().await?;
		da::exec::exec().await?;
		Ok(())
	}
}
