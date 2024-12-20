use clap::Subcommand;

pub mod aws_kms;

#[derive(Subcommand, Debug)]
#[clap(rename_all = "kebab-case", about = "Commands for signing with Secp256k1")]
pub enum Secp256k1 {
	AwsKms(aws_kms::AwsKms),
}

impl Secp256k1 {
	pub async fn run(&self) -> Result<(), anyhow::Error> {
		match self {
			Secp256k1::AwsKms(ak) => ak.run().await,
		}
	}
}
