use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::{BufReader, BufWriter};
use std::path::Path;

pub const WAL_FILE: &str = "wal.json";

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct WalEntry {
                pub operation: String,
                pub canonical_string: String,
                pub status: String,
                pub public_key: Option<String>, // Store the public key in base64 or hex
                pub key_id: Option<String>,    // Store the AWS or Vault Key ID
}

pub fn read_wal() -> Result<Vec<WalEntry>> {
                if !Path::new(WAL_FILE).exists() {
                                return Ok(vec![]);
                }
                let file = fs::File::open(WAL_FILE)?;
                let reader = BufReader::new(file);
                let entries: Vec<WalEntry> = serde_json::from_reader(reader)?;
                Ok(entries)
}

pub fn write_wal(entries: &[WalEntry]) -> Result<()> {
                let file = OpenOptions::new()
                                .create(true)
                                .write(true)
                                .truncate(true)
                                .open(WAL_FILE)?;
                let writer = BufWriter::new(file);
                serde_json::to_writer(writer, entries)?;
                Ok(())
}

pub fn append_to_wal(entry: WalEntry) -> Result<()> {
                let mut entries = read_wal()?;
                entries.push(entry);
                write_wal(&entries)
}

pub fn update_wal_status(canonical_string: &str, new_status: &str) -> Result<()> {
                let mut entries = read_wal()?;
                for entry in &mut entries {
                                if entry.canonical_string == canonical_string {
                                                entry.status = new_status.to_string();
                                }
                }
                write_wal(&entries)
}

pub fn update_wal_entry<F>(canonical_string: &str, update_fn: F) -> Result<()>
where
                F: Fn(&mut WalEntry),
{
                let mut entries = read_wal()?;
                for entry in &mut entries {
                                if entry.canonical_string == canonical_string {
                                                update_fn(entry);
                                }
                }
                write_wal(&entries)
}
