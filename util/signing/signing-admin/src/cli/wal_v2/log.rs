use serde::{Deserialize, Serialize};
use anyhow::{Result, Context};
use signing_admin::{application::Application, backend::SigningBackend, key_manager::KeyManager};
use movement_signer::{
        cryptography::{secp256k1::Secp256k1, ed25519::Ed25519},
        Signing,
};
use movement_signer_aws_kms::hsm::AwsKms;
use movement_signer_hashicorp_vault::hsm::HashiCorpVault;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TransactionId(pub String);

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Operation {
        RotateKey,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StartRotateKey {
        pub key_id: String,
        pub alias: String,
}

impl StartRotateKey {
        pub fn new(alias: String, key_id: String) -> Self {
                Self { key_id, alias }
        }

        pub async fn execute<A, B>(
                &self,
                key_manager: &KeyManager<A, B>,
        ) -> Result<SendAppUpdateKey>
        where
                A: Application,
                B: SigningBackend,
        {
                let new_key_id = key_manager.create_key(&self.alias).await?;
                Ok(SendAppUpdateKey::new(new_key_id, self.alias.clone()))
        }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SendAppUpdateKey {
        pub key_id: String,
        pub alias: String,
}

impl SendAppUpdateKey {
        pub fn new(key_id: String, alias: String) -> Self {
                Self { key_id, alias }
        }

	pub async fn execute<A, B>(
		&self,
		key_manager: &KeyManager<A, B>,
		backend_name: &str,
	) -> Result<RecvAppUpdateKey>
	where
		A: Application,
		B: SigningBackend,
	{
		let public_key = match backend_name {
			"aws" => {
				let aws_config = aws_config::load_from_env().await;
				let client = aws_sdk_kms::Client::new(&aws_config);
				let signer = AwsKms::<Secp256k1>::new(client, self.key_id.clone());
				let raw_public_key = signer
					.public_key()
					.await
					.context("Failed to fetch AWS public key")?;
				raw_public_key.as_bytes().to_vec() // Convert to Vec<u8> to ensure it lives long enough
			}
			"vault" => {
				let vault_url = std::env::var("VAULT_URL")
					.context("Missing VAULT_URL environment variable")?;
				let vault_token = std::env::var("VAULT_TOKEN")
					.context("Missing VAULT_TOKEN environment variable")?;
	
				let vault_client_settings = vaultrs::client::VaultClientSettingsBuilder::default()
					.address(vault_url)
					.token(vault_token)
					.namespace(Some("admin".to_string()))
					.build()
					.context("Failed to build Vault client settings")?;
	
				let client = vaultrs::client::VaultClient::new(vault_client_settings)
					.context("Failed to create Vault client")?;
	
				let signer = HashiCorpVault::<Ed25519>::new(
					client,
					self.key_id.clone(),
					"transit".to_string(),
				);
	
				let raw_public_key = signer
					.public_key()
					.await
					.context("Failed to fetch Vault public key")?;
				raw_public_key.as_bytes().to_vec() // Convert to Vec<u8> to ensure it lives long enough
			}
			_ => return Err(anyhow::anyhow!("Unsupported backend")),
		};
	
		println!("Retrieved public key: {:?}", public_key);
		key_manager
			.notify_application(public_key)
			.await
			.context("Failed to notify application with the public key")?;
		Ok(RecvAppUpdateKey::new(self.key_id.clone(), self.alias.clone()))
	}
			

}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RecvAppUpdateKey {
        pub key_id: String,
        pub alias: String,
}

impl RecvAppUpdateKey {
        pub fn new(key_id: String, alias: String) -> Self {
                Self { key_id, alias }
        }

        pub async fn execute<A, B>(
                &self,
                _key_manager: &KeyManager<A, B>,
        ) -> Result<SendHsmUpdateKey>
        where
                A: Application,
                B: SigningBackend,
        {
                println!(
                        "Executing RecvAppUpdateKey for key_id: {}, alias: {}",
                        self.key_id, self.alias
                );
                Ok(SendHsmUpdateKey::new(self.key_id.clone(), self.alias.clone()))
        }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SendHsmUpdateKey {
        pub key_id: String,
        pub alias: String,
}

impl SendHsmUpdateKey {
        pub fn new(key_id: String, alias: String) -> Self {
                Self { key_id, alias }
        }

        pub async fn execute<A, B>(
                &self,
                key_manager: &KeyManager<A, B>,
        ) -> Result<RecvHsmUpdateKey>
        where
                A: Application,
                B: SigningBackend,
        {
                println!(
                        "Executing SendHsmUpdateKey for key_id: {}, alias: {}",
                        self.key_id, self.alias
                );
                key_manager.rotate_key(&self.key_id).await?;
                Ok(RecvHsmUpdateKey::new(self.key_id.clone(), self.alias.clone()))
        }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RecvHsmUpdateKey {
        pub key_id: String,
        pub alias: String,
}

impl RecvHsmUpdateKey {
        pub fn new(key_id: String, alias: String) -> Self {
                Self { key_id, alias }
        }

        pub async fn execute<A, B>(
                &self,
                _key_manager: &KeyManager<A, B>,
        ) -> Result<()>
        where
                A: Application,
                B: SigningBackend,
        {
                println!(
                        "Executing RecvHsmUpdateKey for key_id: {}, alias: {}",
                        self.key_id, self.alias
                );
                Ok(())
        }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum KeyRotationMessage {
        StartRotateKey(StartRotateKey),
        SendAppUpdateKey(SendAppUpdateKey),
        RecvAppUpdateKey(RecvAppUpdateKey),
        SendHsmUpdateKey(SendHsmUpdateKey),
        RecvHsmUpdateKey(RecvHsmUpdateKey),
}

impl KeyRotationMessage {
        pub async fn execute<A, B>(
                &self,
                key_manager: &KeyManager<A, B>,
                backend_name: &str, // Pass the backend name
        ) -> Result<Self>
        where
                A: Application,
                B: SigningBackend,
        {
                match self {
                        KeyRotationMessage::StartRotateKey(msg) => {
                                msg.execute(key_manager).await.map(KeyRotationMessage::SendAppUpdateKey)
                        }
                        KeyRotationMessage::SendAppUpdateKey(msg) => {
                                msg.execute(key_manager, backend_name) // Pass backend_name here
                                        .await
                                        .map(KeyRotationMessage::RecvAppUpdateKey)
                        }
                        KeyRotationMessage::RecvAppUpdateKey(msg) => {
                                msg.execute(key_manager).await.map(KeyRotationMessage::SendHsmUpdateKey)
                        }
                        KeyRotationMessage::SendHsmUpdateKey(msg) => {
                                msg.execute(key_manager).await.map(KeyRotationMessage::RecvHsmUpdateKey)
                        }
                        KeyRotationMessage::RecvHsmUpdateKey(msg) => {
                                msg.execute(key_manager).await.map(|_| KeyRotationMessage::RecvHsmUpdateKey(msg.clone()))
                        }
                }
        }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct WalEntry {
        pub transaction_id: TransactionId,
        pub operation: Operation,
        pub prepared_messages: Vec<KeyRotationMessage>,
        pub committed_messages: Vec<KeyRotationMessage>,
}

impl WalEntry {
        pub fn new(
                transaction_id: TransactionId,
                operation: Operation,
                prepared_messages: Vec<KeyRotationMessage>,
                committed_messages: Vec<KeyRotationMessage>,
        ) -> Self {
                Self {
                        transaction_id,
                        operation,
                        prepared_messages,
                        committed_messages,
                }
        }

        pub async fn execute<A, B>(
                &mut self,
                key_manager: &KeyManager<A, B>,
                backend_name: &str,
        ) -> Result<()>
        where
                A: Application,
                B: SigningBackend,
        {
                while let Some(message) = self.next_uncommitted_message() {
                        let committed_message = message.execute(key_manager, backend_name).await?;
                        self.committed_messages.push(committed_message);
                }
                Ok(())
        }

        pub fn next_uncommitted_message(&self) -> Option<&KeyRotationMessage> {
                self.prepared_messages.get(self.committed_messages.len())
        }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Wal {
        pub entries: Vec<WalEntry>,
}

impl Wal {
        pub fn new() -> Self {
                Self { entries: vec![] }
        }

        pub fn append(&mut self, entry: WalEntry) {
                self.entries.push(entry);
        }

        pub async fn execute<A, B>(
                &mut self,
                key_manager: &KeyManager<A, B>,
                backend_name: &str,
        ) -> Result<()>
        where
                A: Application,
                B: SigningBackend,
        {
                for entry in &mut self.entries {
                        entry.execute(key_manager, backend_name).await?;
                }
                Ok(())
        }
}