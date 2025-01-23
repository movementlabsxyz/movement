use crate::cli::wal_v2::{
        execute::WalExecutor,
        log::{Wal, WalEntry, Operation, TransactionId},
};
use anyhow::{Context, Result};
use signing_admin::{
        application::{Application, HttpApplication},
        backend::{aws::AwsBackend, vault::VaultBackend, Backend},
        key_manager::KeyManager,
};
use std::{fs, path::Path, io::Write};

const WAL_FILE: &str = "rotate_key.wal";

/// Writes the WAL to a file
fn save_wal_to_file(wal: &Wal) -> Result<()> {
        let serialized_wal = serde_json::to_string(wal).context("Failed to serialize WAL")?;
        let mut file = fs::File::create(WAL_FILE).context("Failed to create WAL file")?;
        file.write_all(serialized_wal.as_bytes()).context("Failed to write WAL to file")?;
        Ok(())
}

/// Reads the WAL from a file
fn load_wal_from_file() -> Result<Wal> {
        let content = fs::read_to_string(WAL_FILE).context("Failed to read WAL file")?;
        let wal = serde_json::from_str(&content).context("Failed to deserialize WAL")?;
        Ok(wal)
}

pub async fn rotate_key(
        canonical_string: String,
        application_url: String,
        backend_name: String, // Already here
) -> Result<()> {
        // Check if the WAL file exists
        if Path::new(WAL_FILE).exists() {
                return Err(anyhow::anyhow!(
                        "WAL file exists. Use the recover command to recover or clean up the WAL."
                ));
        }

        // Initialize the application and backend
        let application = HttpApplication::new(application_url.clone());
        let backend = match backend_name.as_str() {
                "vault" => Backend::Vault(VaultBackend::new()),
                "aws" => Backend::Aws(AwsBackend::new()),
                _ => return Err(anyhow::anyhow!("Unsupported backend: {}", backend_name)),
        };

        // Create the key manager
        let key_manager = KeyManager::new(application, backend);

        // Initialize the WAL executor
        let executor = WalExecutor::new();

        // Initialize a new WAL
        let mut wal = Wal::new();

        // Create a new key
        let new_key_id = key_manager.create_key(&canonical_string).await
                .context("Failed to create a new key")?;

        // Append the WAL entry
        wal = executor
                .append(wal, canonical_string.clone(), new_key_id.clone(), &key_manager, &backend_name)
                .await
                .map_err(|(wal, err)| {
                        anyhow::anyhow!("Failed to append to WAL: {:?}", err)
                                .context(format!("Error occurred with WAL: {:?}", wal))
                })?;

        // Save the WAL to a file
        save_wal_to_file(&wal).context("Failed to save WAL to file")?;

        // Execute the WAL
        wal = executor
                .execute(wal, &key_manager, &backend_name) // Pass backend_name
                .await
                .map_err(|(wal, err)| {
                        anyhow::anyhow!("Failed to execute WAL: {:?}", err)
                                .context(format!("Error occurred with WAL: {:?}", wal))
                })?;

        // Clean up after successful execution
        fs::remove_file(WAL_FILE).context("Failed to clean up WAL file after successful execution")?;
        println!("Key rotation completed successfully for key: {}", canonical_string);

        Ok(())
}

