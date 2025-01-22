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
		let address = std::env::var("VAULT_ADDR").context("VAULT_ADDR not set")?;
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
			self.key_name.replace("/", "_").as_str(),
			base64::encode(message).as_str(),
			None,
		)
		.await
		.map_err(|e| {
			let error_msg = format!("Failed to sign message: {:?}", e);
			println!("{}", error_msg);
			SignerError::Internal(error_msg)
		})?;

		// Log the full response
		println!("Signature response: {:?}", res);

		if !res.signature.starts_with("vault") {
			return Err(SignerError::Internal("Invalid signature format".to_string()));
		}

		// Extract the key version from the signature
		let version_end_index = res.signature[6..]
			.find(':')
			.ok_or_else(|| SignerError::Internal("Invalid signature format".to_string()))?
			+ 6;

		// Determine split index dynamically
		let split_index = version_end_index + 1;

		// Split and decode the signature
		let signature_str = &res.signature[split_index..];

		let signature = base64::decode(signature_str)
			.context("Failed to decode base64 signature from Vault")
			.map_err(|e| SignerError::Internal(e.to_string()))?;

		if signature.len() != 64 {
			return Err(SignerError::Internal(format!(
				"Unexpected signature length: {} bytes",
				signature.len()
			)));
		}

		let parsed_signature = C::Signature::try_from_bytes(&signature).map_err(|e| {
			SignerError::Internal(format!(
				"Failed to parse signature into expected format: {:?}",
				e
			))
		})?;

		Ok(parsed_signature)
	}

	async fn public_key(&self) -> Result<C::PublicKey, SignerError> {
		println!("Attempting to read Vault key: {}", self.key_name);

		// Read the key from Vault
		let res = transit_key::read(
			&self.client,
			self.mount_name.as_str(),
			self.key_name.replace("/", "_").as_str(),
		)
		.await
		.map_err(|e| {
			println!(
				"Error reading key '{}' from mount '{}': {:?}",
				self.key_name, self.mount_name, e
			);
			SignerError::Internal(format!("Failed to read key '{}': {:?}", self.key_name, e))
		})?;

		println!("Key read successfully: {:?}", res);

		// Match the key type and determine the latest version
		let public_key = match res.keys {
			ReadKeyData::Symmetric(_) => {
				println!("Key '{}' is symmetric and not supported", self.key_name);
				return Err(SignerError::Internal("Symmetric keys are not supported".to_string()));
			}
			ReadKeyData::Asymmetric(keys) => {
				// Use the number of items in the map as the version
				let latest_version = keys.len().to_string();

				let key =
					keys.get(&latest_version).context("Key version not found").map_err(|e| {
						println!("Key version '{}' not found: {:?}", latest_version, e);
						SignerError::KeyNotFound
					})?;

				base64::decode(&key.public_key).map_err(|e| {
					println!("Failed to decode public key: {:?}", e);
					SignerError::Internal(e.to_string())
				})?
			}
		};

		Ok(C::PublicKey::try_from_bytes(&public_key).map_err(|e| {
			println!("Error converting public key to curve type: {:?}. Bytes: {:?}", e, public_key);
			SignerError::Internal(e.to_string())
		})?)
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
