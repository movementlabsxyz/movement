use crate::admin::l1_migration::validate::types::da::get_da_block_height;
use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[clap(name = "da-height", about = "Extract synced block height from the DA-sequencer database")]
pub struct DaHeight {
	#[arg(help = "Path to the DA-Sequencer database")]
	path: PathBuf,
}

impl DaHeight {
	pub fn run(&self) -> anyhow::Result<()> {
		let height = get_da_block_height(&self.path)?;
		println!("{}", height);
		Ok(())
	}
}

#[test]
fn verify_tool() {
	use clap::CommandFactory;
	DaHeight::command().debug_assert()
}
