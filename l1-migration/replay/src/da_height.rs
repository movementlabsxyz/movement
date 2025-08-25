use crate::types::da::DaSequencerDb;
use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
#[clap(name = "da-height", about = "Extract synced block height from the DA-sequencer database")]
pub struct DaHeight {
	#[arg(long, help = "Path to the DA-Sequencer database")]
	path: PathBuf,
}

impl DaHeight {
	pub fn run(self) -> anyhow::Result<()> {
		let db = DaSequencerDb::open(self.path)?;
		let height = db.get_synced_height()?;
		println!("{}", height);
		Ok(())
	}
}

#[test]
fn verify_tool() {
	use clap::CommandFactory;
	DaHeight::command().debug_assert()
}
