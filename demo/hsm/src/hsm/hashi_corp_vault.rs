use crate::{Bytes, Hsm, Signature};
use vaultrs::api::transit::requests::CreateKeyRequest;
use vaultrs::api::transit::KeyType;
use vaultrs::client::{VaultClient, VaultClientSettingsBuilder};
use vaultrs::transit::data;
use vaultrs::transit::key;

/// A HashiCorp Vault HSM.
pub struct HashiCorpVault {
	client: VaultClient,
	key_name: String,
	mount_name: String,
}

impl HashiCorpVault {
	/// Creates a new HashiCorp Vault HSM
	pub fn new(client: VaultClient, key_name: String, mount_name: String) -> Self {
		Self { client, key_name, mount_name }
	}

	/// Tries to create a new HashiCorp Vault HSM from the environment
	pub fn try_from_env() -> Result<Self, anyhow::Error> {
		let address = std::env::var("VAULT_ADDR").unwrap_or_else(|_| "https://

		let client = VaultClient::new(
			VaultClientSettingsBuilder::default()
				.address("https://127.0.0.1:8200")
				.token("TOKEN")
				.build()?,
		)?;
		let key_name = std::env::var("VAULT_KEY_NAME")?;
		let mount_name = std::env::var("VAULT_MOUNT_NAME")?;

		Ok(Self::new(client, key_name, mount_name))
	}

	/// Creates a new key in the transit backend
	pub async fn new_key(self) -> Result<(), anyhow::Error> {
		key::create(
			&self.client,
			self.mount_name.as_str(),
			self.key_name.as_str(),
			Some(CreateKeyRequest::builder().key_type(KeyType::Ed25519)),
		)
		.await?;

		Ok(())
	}
}

#[async_trait::async_trait]
impl Hsm for HashiCorpVault {
	async fn sign(&self, message: Bytes) -> Result<Signature, anyhow::Error> {
		let res = data::sign(
			&self.client,
			self.mount_name.as_str(),
			self.key_name.as_str(),
			// convert bytes vec<u8> to base64 string
			base64::encode(message.0).as_str(),
			None,
		)
		.await?;

		// decode base64 string to vec<u8>
		let signature = base64::decode(res.signature)?;

		// Sign the message using HashiCorp Vault
		Ok(Signature(Bytes(signature)))
	}

	async fn verify(&self, _message: Bytes, _signature: Signature) -> Result<bool, anyhow::Error> {
		Ok(true)
	}
}
