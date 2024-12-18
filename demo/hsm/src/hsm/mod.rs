pub mod aws_kms;
pub mod google_kms;
pub mod hashi_corp_vault;

#[derive(Debug, Clone, Copy)]
pub enum Provider {
	AWS,
	GCP,
	Vault,
}
