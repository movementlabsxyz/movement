pub mod bucket;
use clap::Subcommand;

#[derive(Subcommand, Debug)]
#[clap(rename_all = "kebab-case", about = "Control the syncing")]
pub enum Syncing {
	#[clap(subcommand)]
	Bucket(bucket::Bucket),
}
