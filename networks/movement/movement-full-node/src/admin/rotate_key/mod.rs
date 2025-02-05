use clap::Subcommand;
pub mod core_resource_account;
pub mod known_signer;
pub mod mcr_validator;

#[derive(Subcommand, Debug)]
#[clap(rename_all = "kebab-case", about = "Commands for rotating keys")]
pub enum RotateKey {
	CoreResourceAccount(core_resource_account::CoreResourceAccount),
	KnownSigner(known_signer::KnownSigner),
	McrValidator(mcr_validator::McrValidator),
}

impl RotateKey {
	pub async fn execute(&self) -> Result<(), anyhow::Error> {
		match self {
			RotateKey::CoreResourceAccount(core_resource_account) => {
				core_resource_account.execute().await
			}
			RotateKey::KnownSigner(known_signer) => known_signer.execute().await,
			RotateKey::McrValidator(mcr_validator) => mcr_validator.execute().await,
		}
	}
}
