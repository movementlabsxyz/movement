use anyhow::{Context, Result};
use movement_signer::{
	cryptography::{ed25519::Ed25519, secp256k1::Secp256k1},
	Signing,
};
use movement_signer_aws_kms::hsm::AwsKms;
use movement_signer_hashicorp_vault::hsm::HashiCorpVault;
use signing_admin::{
	application::{Application, HttpApplication},
	backend::{aws::AwsBackend, vault::VaultBackend, Backend},
	key_manager::KeyManager,
};
use vaultrs::client::{VaultClient, VaultClientSettingsBuilder};

/// Enum to encapsulate different signers
enum SignerBackend {
	Vault(HashiCorpVault<Ed25519>),
	Aws(AwsKms<Secp256k1>),
}

impl SignerBackend {
	/// Retrieve the public key from the signer
	async fn public_key(&self) -> Result<Vec<u8>> {
		match self {
			SignerBackend::Vault(signer) => {
				let public_key = signer.public_key().await?;
				Ok(public_key.as_bytes().to_vec())
			}
			SignerBackend::Aws(signer) => {
				let public_key = signer.public_key().await?;
				Ok(public_key.as_bytes().to_vec())
			}
		}
	}
}

pub async fn rotate_key(
	canonical_string: String,
	application_url: String,
	backend_name: String,
) -> Result<()> {
	let application = HttpApplication::new(application_url);

	let backend = match backend_name.as_str() {
		"vault" => Backend::Vault(VaultBackend::new()),
		"aws" => Backend::Aws(AwsBackend::new()),
		_ => return Err(anyhow::anyhow!("Unsupported backend: {}", backend_name)),
	};

	let signer = match backend_name.as_str() {
		"vault" => {
			let vault_url =
				std::env::var("VAULT_URL").context("Missing VAULT_URL environment variable")?;
			let vault_token =
				std::env::var("VAULT_TOKEN").context("Missing VAULT_TOKEN environment variable")?;

			let client = VaultClient::new(
				VaultClientSettingsBuilder::default()
					.address(vault_url)
					.token(vault_token)
					.namespace(Some("admin".to_string()))
					.build()
					.context("Failed to build Vault client settings")?,
			)
			.context("Failed to create Vault client")?;

			SignerBackend::Vault(HashiCorpVault::<Ed25519>::new(
				client,
				canonical_string.clone(),
				"transit".to_string(),
			))
		}
		"aws" => {
			let aws_config = aws_config::load_from_env().await;
			let client = aws_sdk_kms::Client::new(&aws_config);

			SignerBackend::Aws(AwsKms::<Secp256k1>::new(client, canonical_string.clone()))
		}
		_ => return Err(anyhow::anyhow!("Unsupported signer backend: {}", backend_name)),
	};

	let key_manager = KeyManager::new(application, backend);

	key_manager
		.rotate_key(&canonical_string)
		.await
		.context("Failed to rotate the key")?;

	let public_key = signer
		.public_key()
		.await
		.context("Failed to fetch the public key from signer")?;

	key_manager
		.application
		.notify_public_key(public_key)
		.await
		.context("Failed to notify the application with the public key")?;

	println!("Key rotation and notification completed successfully.");
	Ok(())
}
