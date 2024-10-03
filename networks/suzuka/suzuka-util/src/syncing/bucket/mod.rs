pub mod delete;
pub mod downsync;
pub mod upsync;

use clap::Subcommand;

#[derive(Subcommand, Debug)]
#[clap(rename_all = "kebab-case", about = "Control bucket-based syncing")]
pub enum Bucket {
	Delete(delete::Delete),
	Downsync(downsync::Downsync),
	Upsync(upsync::Upsync),
}
