use crate::cryptography::hashicorp_vault::HashiCorpVaultCryptography;
use crate::cryptography::verifier::LocalVerifier;
use crate::{Bytes, Hsm, PublicKey, Signature};
use anyhow::Context;
use vaultrs::api::transit::{requests::CreateKeyRequest, responses::ReadKeyData};
use vaultrs::client::{VaultClient, VaultClientSettingsBuilder};
use vaultrs::transit::data;
use vaultrs::transit::key;

/// A HashiCorp Vault HSM.
pub struct HashiCorpVault<C: HashiCorpVaultCryptography> {
	client: VaultClient,
	key_name: String,
	mount_name: String,
	pub public_key: PublicKey,
	_cryptography_marker: std::marker::PhantomData<C>,
}

impl<C> HashiCorpVault<C>
where
	C: HashiCorpVaultCryptography,
{
	/// Creates a new HashiCorp Vault HSM
	pub fn new(
		client: VaultClient,
		key_name: String,
		mount_name: String,
		public_key: PublicKey,
	) -> Self {
		Self {
			client,
			key_name,
			mount_name,
			public_key,
			_cryptography_marker: std::marker::PhantomData,
		}
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
		let public_key = std::env::var("VAULT_PUBLIC_KEY").unwrap_or_default();

		Ok(Self::new(
			client,
			key_name,
			mount_name,
			PublicKey(Bytes(public_key.as_bytes().to_vec())),
		))
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

	/// Fills with a public key fetched from vault.
	pub async fn fill_with_public_key(self) -> Result<Self, anyhow::Error> {
		let res = key::read(&self.client, self.mount_name.as_str(), self.key_name.as_str())
			.await
			.context("Failed to read key")?;
		println!("Read key: {:?}", res);

		let public_key = match res.keys {
			ReadKeyData::Symmetric(_) => {
				return Err(anyhow::anyhow!("Symmetric keys are not supported"));
			}
			ReadKeyData::Asymmetric(keys) => {
				let key = keys.values().next().context("No key found")?;
				base64::decode(key.public_key.as_str()).context("Failed to decode public key")?
			}
		};

		println!("Public key: {:?}", public_key);
		Ok(Self::new(self.client, self.key_name, self.mount_name, PublicKey(Bytes(public_key))))
	}
}

#[async_trait::async_trait]
impl<C> Hsm for HashiCorpVault<C>
where
	C: HashiCorpVaultCryptography + LocalVerifier + Send + Sync,
{
	async fn sign(&self, message: Bytes) -> Result<(Bytes, PublicKey, Signature), anyhow::Error> {
		let res = data::sign(
			&self.client,
			self.mount_name.as_str(),
			self.key_name.as_str(),
			// convert bytes vec<u8> to base64 string
			base64::encode(message.clone().0).as_str(),
			None,
		)
		.await
		.context("Failed to sign message")?;

		// the signature should be encoded valut:v1:<signature> check for match and split off the signature
		// 1. check for match
		if !res.signature.starts_with("vault:v1:") {
			return Err(anyhow::anyhow!("Invalid signature format"));
		}
		// 2. split off the signature
		let signature_str = res.signature.split_at(9).1;

		// decode base64 string to vec<u8>
		let signature = base64::decode(signature_str).context("Failed to decode signature")?;

		// Sign the message using HashiCorp Vault
		Ok((message, self.public_key.clone(), Signature(Bytes(signature))))
	}

	async fn verify(
		&self,
		message: Bytes,
		public_key: PublicKey,
		signature: Signature,
	) -> Result<bool, anyhow::Error> {
		C::verify(message, public_key, signature).await
	}
}
