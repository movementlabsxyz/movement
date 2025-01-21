pub mod identifiers;

use identifiers::SignerIdentifier;
use movement_signer::key::SignerBuilder;
use movement_signer::{
	cryptography::{ed25519::Ed25519, secp256k1::Secp256k1, Curve},
	Signing,
};
use std::sync::Arc;
use tracing::debug;
use tracing::info;

/// A signer loaded dynamically.
#[derive(Clone)]
pub struct LoadedSigner<C>
where
	C: Curve,
{
	signer: Arc<dyn Signing<C> + Send + Sync>,
}

impl<C> LoadedSigner<C>
where
	C: Curve,
{
	pub fn new(signer: Arc<dyn Signing<C> + Send + Sync>) -> Self {
		Self { signer }
	}
}

#[async_trait::async_trait]
impl<C> Signing<C> for LoadedSigner<C>
where
	C: Curve,
{
	async fn sign(
		&self,
		message: &[u8],
	) -> Result<<C as Curve>::Signature, movement_signer::SignerError> {
		debug!("using a loaded signer to sign a message");
		self.signer.sign(message).await
	}

	async fn public_key(&self) -> Result<<C as Curve>::PublicKey, movement_signer::SignerError> {
		debug!("using a loaded signer to get the public key");
		self.signer.public_key().await
	}
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
#[async_trait::async_trait]
pub trait Load<C>
where
	C: Curve,
{
	/// Loads the signer.
	async fn load(&self) -> Result<LoadedSigner<C>, LoaderError>;
}

#[async_trait::async_trait]
impl Load<Secp256k1> for SignerIdentifier {
	async fn load(&self) -> Result<LoadedSigner<Secp256k1>, LoaderError> {
		info!("loading a secp256k1 signer {:?}", self);
		match self {
			SignerIdentifier::Local(local) => {
				let signer = movement_signer_local::signer::LocalSigner::from_signing_key_hex(
					&local.private_key_hex_bytes,
				)
				.map_err(|_e| LoaderError::InvalidSigner)?;
				Ok(LoadedSigner::new(Arc::new(signer) as Arc<dyn Signing<Secp256k1> + Send + Sync>))
			}
			SignerIdentifier::AwsKms(aws_kms) => {
				let builder =
					movement_signer_aws_kms::hsm::key::Builder::new().create_key(aws_kms.create);
				let key = aws_kms.key.clone();
				let signer = builder.build(key).await.map_err(|_e| LoaderError::InvalidSigner)?;
				Ok(LoadedSigner::new(Arc::new(signer) as Arc<dyn Signing<Secp256k1> + Send + Sync>))
			}
			SignerIdentifier::HashiCorpVault(_hashi_corp_vault) => Err(LoaderError::InvalidCurve),
		}
	}
}

#[async_trait::async_trait]
impl Load<Ed25519> for SignerIdentifier {
	async fn load(&self) -> Result<LoadedSigner<Ed25519>, LoaderError> {
		info!("loading an ed25519 signer {:?}", self);
		match self {
			SignerIdentifier::Local(_local) => Err(LoaderError::InvalidCurve),
			SignerIdentifier::AwsKms(_aws_kms) => Err(LoaderError::InvalidCurve),
			SignerIdentifier::HashiCorpVault(hashi_corp_vault) => {
				let builder = movement_signer_hashicorp_vault::hsm::key::Builder::new()
					.create_key(hashi_corp_vault.create);
				let key = hashi_corp_vault.key.clone();
				let signer = builder.build(key).await.map_err(|_e| LoaderError::InvalidSigner)?;
				Ok(LoadedSigner::new(Arc::new(signer) as Arc<dyn Signing<Ed25519> + Send + Sync>))
			}
		}
	}
}
