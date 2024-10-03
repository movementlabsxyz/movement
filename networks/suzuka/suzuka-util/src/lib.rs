pub mod common_args;
pub mod syncing;
use clap::Parser;

#[derive(Parser)]
#[clap(rename_all = "kebab-case")]
pub enum SuzukaUtil {
	#[clap(subcommand)]
	Syncing(syncing::Syncing),
}
