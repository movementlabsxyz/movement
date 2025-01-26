use clap::Subcommand;

#[derive(Subcommand, Debug)]
#[clap(rename_all = "kebab-case", about = "Commands for bespoke network operations")]
pub enum Ops {}

impl Ops {
	pub async fn execute(&self) -> Result<(), anyhow::Error> {
		match self {}
	}
}
