use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Serialize, Deserialize)]
pub struct SwapState {
	pub id: String,
	pub recipient: String,
	pub amount: u64,
	pub status: SwapStatus,
}

#[derive(Serialize, Deserialize)]
pub enum SwapStatus {
	Initiated,
	Completed,
	Failed,
}

fn get_state_dir() -> Result<PathBuf> {
	let proj_dirs = ProjectDirs::from("xyz", "movementlabs", "bridge-cli")
		.context("Failed to get project directories")?;
	let state_dir = proj_dirs.data_local_dir();
	fs::create_dir_all(state_dir)?;
	Ok(state_dir.to_path_buf())
}

pub fn save_swap_state(state: &SwapState) -> Result<()> {
	let state_dir = get_state_dir()?;
	let file_path = state_dir.join(format!("{}.json", state.id));
	let json = serde_json::to_string_pretty(state)?;
	fs::write(file_path, json)?;
	Ok(())
}

pub fn load_swap_state(id: &str) -> Result<SwapState> {
	let state_dir = get_state_dir()?;
	let file_path = state_dir.join(format!("{}.json", id));
	let json = fs::read_to_string(file_path)?;
	let state: SwapState = serde_json::from_str(&json)?;
	Ok(state)
}
