use crate::cryptography::HashiCorpVaultCryptography;
use anyhow::Context;
use movement_signer::{
	cryptography::{
		ed25519::{self, Ed25519},
		Curve,
	},
	SignerError, Signing,
};
use vaultrs::api::transit::{requests::CreateKeyRequest, responses::ReadKeyData};
use vaultrs::client::{VaultClient, VaultClientSettingsBuilder};
use vaultrs::transit::key;

/// A HashiCorp Vault HSM.
pub struct HashiCorpVault<C: Curve + HashiCorpVaultCryptography> {
	client: VaultClient,
	key_name: String,
	mount_name: String,
	_cryptography_marker: std::marker::PhantomData<C>,
}

impl<C> HashiCorpVault<C>
where
	C: Curve + HashiCorpVaultCryptography,
{
	/// Creates a new HashiCorp Vault HSM
	pub fn new(client: VaultClient, key_name: String, mount_name: String) -> Self {
		Self { client, key_name, mount_name, _cryptography_marker: std::marker::PhantomData }
	}

	/// Tries to create a new HashiCorp Vault HSM from the environment
	pub fn try_from_env() -> Result<Self, anyhow::Error> {
		let address = std::env::var("VAULT_ADDRESS").context("VAULT_ADDRESS not set")?;
		let token = std::env::var("VAULT_TOKEN").context("VAULT_TOKEN not set")?;
		let namespace = std::env::var("VAULT_NAMESPACE").unwrap_or_else(|_| "admin".to_string());
		let client = VaultClient::new(
			VaultClientSettingsBuilder::default()
				.address(address.as_str())
				.token(token.as_str())
				.namespace(Some(namespace))
				.build()?,
		)?;

		let key_name = std::env::var("VAULT_KEY_NAME").context("VAULT_KEY_NAME not set")?;
		let mount_name = std::env::var("VAULT_MOUNT_NAME").context("VAULT_MOUNT_NAME not set")?;

		Ok(Self::new(client, key_name, mount_name))
	}

	/// Creates a new key in the transit backend
	pub async fn create_key(self) -> Result<Self, anyhow::Error> {
		key::create(
			&self.client,
			self.mount_name.as_str(),
			self.key_name.as_str(),
			Some(CreateKeyRequest::builder().key_type(C::key_type()).derived(false)),
		)
		.await
		.context("Failed to create key")?;
		Ok(self)
	}
}

impl Signing<Ed25519> for HashiCorpVault<Ed25519> {
	async fn sign(&self, _message: &[u8]) -> Result<<Ed25519 as Curve>::Signature, SignerError> {
		unimplemented!()
	}

	async fn public_key(&self) -> Result<<Ed25519 as Curve>::PublicKey, SignerError> {
		let res = key::read(&self.client, self.mount_name.as_str(), self.key_name.as_str())
			.await
			.context("Failed to read key")
			.map_err(|e| SignerError::Internal(e.to_string()))?;
		println!("Read key: {:?}", res);

		let public_key = match res.keys {
			ReadKeyData::Symmetric(_) => {
				return Err(SignerError::Internal("Symmetric keys are not supported".to_string()));
			}
			ReadKeyData::Asymmetric(keys) => {
				let key = keys.values().next().ok_or_else(|| SignerError::KeyNotFound)?;
				base64::decode(key.public_key.as_str())
					.context("failed to decode public key")
					.map_err(|e| SignerError::Decode(e.into()))?
			}
		};

		Ok(ed25519::PublicKey::try_from(public_key.as_slice())
			.map_err(|e| SignerError::Decode(e.into()))?)
	}
}
