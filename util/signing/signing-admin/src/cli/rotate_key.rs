use crate::cli::wal::{append_to_wal, update_wal_entry, update_wal_status, WalEntry, WAL_FILE};
use anyhow::{Context, Result};
use hex;
use movement_signer::{
	cryptography::{ed25519::Ed25519, secp256k1::Secp256k1},
	Signing,
};
use signing_admin::{
	application::{Application, HttpApplication},
	backend::{aws::AwsBackend, vault::VaultBackend, Backend},
	key_manager::KeyManager,
};
use std::{fs, path::Path};
pub async fn rotate_key(
	canonical_string: String,
	application_url: String,
	backend_name: String,
) -> Result<()> {
	// Clean up existing WAL file
	if Path::new(WAL_FILE).exists() {
		fs::remove_file(WAL_FILE).context("Failed to clean WAL file")?;
	}
	// Initialize the application
	let application = HttpApplication::new(application_url);

	// Initialize the backend
	let backend = match backend_name.as_str() {
		"vault" => Backend::Vault(VaultBackend::new()),
		"aws" => Backend::Aws(AwsBackend::new()),
		_ => return Err(anyhow::anyhow!("Unsupported backend: {}", backend_name)),
	};

	// Create the key manager
	let key_manager = KeyManager::new(application, backend);

	// Phase 1: Create new key and prepare application
	let new_key_id = key_manager.create_key(&canonical_string).await?;
	append_to_wal(WalEntry {
		operation: "rotate_key".to_string(),
		canonical_string: canonical_string.clone(),
		status: "key_created".to_string(),
		public_key: None,
		key_id: Some(new_key_id.clone()),
	})?;
	update_wal_status(&canonical_string, "key_created")?;

	let public_key = new_key_id.as_bytes().to_vec();
	key_manager
		.notify_application(public_key.clone())
		.await
		.context("Failed to notify application with the public key")?;
	update_wal_entry(&canonical_string, |entry| {
		entry.public_key = Some(hex::encode(&public_key));
	})?;
	update_wal_status(&canonical_string, "app_public_key_updated")?;

	// Phase 2: Rotate Key
	update_wal_status(&canonical_string, "commit")?;
	key_manager
		.rotate_key(&new_key_id)
		.await
		.context("Failed to rotate key to the new ID")?;
	update_wal_status(&canonical_string, "completed")?;

	println!("Key rotation completed successfully.");

	fs::remove_file(WAL_FILE).context("Failed to delete WAL file after successful operation")?;

	Ok(())
}
