pub mod aws_kms;
pub mod hashi_corp_vault;
pub mod local;

use movement_signer::{
	cryptography::{ed25519::Ed25519, secp256k1::Secp256k1, Curve},
	Signing,
};
use serde::{Deserialize, Serialize};
use std::future::Future;

#[derive(Debug, Serialize, Deserialize)]
pub enum SignerIdentifier {
	Local(local::Local),
	AwsKms(aws_kms::AwsKms),
	HashiCorpVault(hashi_corp_vault::HashiCorpVault),
}

/// Errors thrown by Signer
#[derive(Debug, thiserror::Error)]
pub enum LoaderError {
	#[error("Invalid signer identifier")]
	InvalidSignerIdentifier,
	#[error("Invalid signer")]
	InvalidSigner,
	#[error("Invalid curve")]
	InvalidCurve,
}

/// Loads a signer.
///
/// The curve for the signer should always be statically known by the application.
pub trait Load<C>
where
	C: Curve,
{
	fn load(&self) -> impl Future<Output = Result<Box<dyn Signing<C>>, LoaderError>> + Send;
}

impl Load<Secp256k1> for SignerIdentifier {
	async fn load(&self) -> Result<Box<dyn Signing<Secp256k1>>, LoaderError> {
		match self {
			SignerIdentifier::Local(local) => Err(LoaderError::InvalidCurve),
			SignerIdentifier::AwsKms(aws_kms) => {
				let builder = movement_signer_aws_kms::hsm::key::Builder::new();
				let key = aws_kms.key.clone();
				let signer = builder.build(key).await?;
				Ok(Box::new(signer) as Box<dyn Signing<Secp256k1>>)
			}
			SignerIdentifier::HashiCorpVault(hashi_corp_vault) => Err(LoaderError::InvalidCurve),
		}
	}
}

impl Load<Ed25519> for SignerIdentifier {
	async fn load(&self) -> Result<Box<dyn Signing<Ed25519>>, LoaderError> {
		match self {
			SignerIdentifier::Local(local) => Err(LoaderError::InvalidCurve),
			SignerIdentifier::AwsKms(aws_kms) => Err(LoaderError::InvalidCurve),
			SignerIdentifier::HashiCorpVault(hashi_corp_vault) => {
				let builder = movement_signer_hashi_corp_vault::hsm::key::Builder::new();
				let key = hashi_corp_vault.key.clone();
				let signer = builder.build(key).await?;
				Ok(Box::new(signer) as Box<dyn Signing<Ed25519>>)
			}
		}
	}
}
