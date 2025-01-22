pub mod key;

use crate::cryptography::HashiCorpVaultCryptographySpec;
use anyhow::Context;
use movement_signer::cryptography::TryFromBytes;
use movement_signer::{cryptography::Curve, SignerError, Signing};
use vaultrs::api::transit::{requests::CreateKeyRequest, responses::ReadKeyData};
use vaultrs::client::{VaultClient, VaultClientSettingsBuilder};
use vaultrs::transit::data;
use vaultrs::transit::key as transit_key;

/// A HashiCorp Vault HSM.
pub struct HashiCorpVault<C: Curve + HashiCorpVaultCryptographySpec> {
	client: VaultClient,
	key_name: String,
	mount_name: String,
	_cryptography_marker: std::marker::PhantomData<C>,
}

impl<C> HashiCorpVault<C>
where
	C: Curve + HashiCorpVaultCryptographySpec,
{
	/// Creates a new HashiCorp Vault HSM
	pub fn new(client: VaultClient, key_name: String, mount_name: String) -> Self {
		Self { client, key_name, mount_name, _cryptography_marker: std::marker::PhantomData }
	}

	/// Sets the key id
	pub fn set_key_id(&mut self, key_id: String) {
		self.key_name = key_id;
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

		let key_name = std::env::var("VAULT_KEY_NAME").unwrap_or_else(|_| "signer".to_string());
		let mount_name = std::env::var("VAULT_MOUNT_NAME").context("VAULT_MOUNT_NAME not set")?;

		Ok(Self::new(client, key_name, mount_name))
	}

	/// Creates a random key using env configuration, but replacing the key name with a random one
	pub async fn create_random_key() -> Result<Self, anyhow::Error> {
		let mut hsm = Self::try_from_env()?;
		hsm.key_name = format!("key-{}", uuid::Uuid::new_v4().to_string());
		Ok(hsm.create_key().await?)
	}

	/// Creates a new key in the transit backend
	pub async fn create_key(self) -> Result<Self, anyhow::Error> {
		transit_key::create(
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

#[async_trait::async_trait]
impl<C> Signing<C> for HashiCorpVault<C>
where
	C: Curve + HashiCorpVaultCryptographySpec + Sync,
{
	async fn sign(&self, message: &[u8]) -> Result<C::Signature, SignerError> {
		println!("Key name: {:?}", self.key_name.as_str());
		let res = data::sign(
			&self.client,
			self.mount_name.as_str(),
			self.key_name.as_str(),
			// convert bytes vec<u8> to base64 string
			base64::encode(message).as_str(),
			None,
		)
		.await
		.context("Failed to sign message")
		.map_err(|e| SignerError::Internal(e.to_string()))?;

		// the signature should be encoded vault:v1:<signature> check for match and split off the signature
		// 1. check for match
		if !res.signature.starts_with("vault:v1:") {
			return Err(SignerError::Internal("Invalid signature format".to_string()));
		}
		// 2. split off the signature
		let signature_str = res.signature.split_at(9).1;

		// decode base64 string to vec<u8>
		let signature = base64::decode(signature_str)
			.context("Failed to decode signature")
			.map_err(|e| SignerError::Internal(e.to_string()))?;

		// Sign the message using HashiCorp Vault
		Ok(C::Signature::try_from_bytes(signature.as_slice())
			.map_err(|e| SignerError::Internal(e.to_string()))?)
	}

	async fn public_key(&self) -> Result<C::PublicKey, SignerError> {
		let res = transit_key::read(&self.client, self.mount_name.as_str(), self.key_name.as_str())
			.await
			.context("Failed to read key")
			.map_err(|e| SignerError::Internal(e.to_string()))?;

		let public_key = match res.keys {
			ReadKeyData::Symmetric(_) => {
				return Err(SignerError::Internal("Symmetric keys are not supported".to_string()));
			}
			ReadKeyData::Asymmetric(keys) => {
				let key = keys
					.values()
					.next()
					.context("No key found")
					.map_err(|_e| SignerError::KeyNotFound)?;
				base64::decode(key.public_key.as_str())
					.context("failed to decode public key")
					.map_err(|e| SignerError::Decode(e.into()))?
			}
		};

		Ok(C::PublicKey::try_from_bytes(public_key.as_slice())
			.map_err(|e| SignerError::Internal(e.to_string()))?)
	}
}

#[cfg(test)]
pub mod test {

	use super::*;
	use movement_signer::{cryptography::ed25519::Ed25519, Signing, Verify};

	#[tokio::test]
	async fn test_signs_and_verifies_ed25519() -> Result<(), anyhow::Error> {
		// load with dotenv
		dotenv::dotenv().ok();

		let hsm = HashiCorpVault::<Ed25519>::create_random_key().await?;
		let message = b"hello world";
		let signature = hsm.sign(message).await?;
		let public_key = hsm.public_key().await?;

		assert!(Ed25519::verify(message, &signature, &public_key)?);

		Ok(())
	}
}
