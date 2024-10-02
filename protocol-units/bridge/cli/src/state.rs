/// Currently unused
use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::{fs, path::Path};

#[derive(Serialize, Deserialize)]
pub struct SwapState {
	pub id: String,
	pub swap_type: SwapType,
	pub block_height: u64,
	pub block_height_timeout: u64,
	pub recipient: String,
	pub amount: u64,
	pub status: SwapStatus,
}

#[derive(Serialize, Deserialize)]
pub enum SwapType {
	EthToMovement,
	MovementToEth,
}

impl std::fmt::Display for SwapType {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			SwapType::EthToMovement => write!(f, "eth_to_movement"),
			SwapType::MovementToEth => write!(f, "movement_to_eth"),
		}
	}
}
impl std::convert::AsRef<Path> for SwapType {
	fn as_ref(&self) -> &Path {
		match self {
			SwapType::EthToMovement => Path::new("eth_to_movement"),
			SwapType::MovementToEth => Path::new("movement_to_eth"),
		}
	}
}

#[derive(Serialize, Deserialize)]
pub enum SwapStatus {
	Initiated,
	Completed,
	Failed,
}

fn ensure_state_dir() -> Result<PathBuf> {
	let proj_dirs = ProjectDirs::from("xyz", "movementlabs", "bridge-cli")
		.context("Failed to get project directories")?;
	let state_dir = proj_dirs.data_local_dir();
	fs::create_dir_all(state_dir)?;
	Ok(state_dir.to_path_buf())
}

pub fn save_swap_state(state: &SwapState) -> Result<()> {
	let state_dir = ensure_state_dir()?;
	let file_path = state_dir.join(&state.swap_type).join(format!("{}.json", state.id));
	let json = serde_json::to_string_pretty(state)?;
	fs::write(file_path, json)?;
	Ok(())
}

pub fn load_swap_state(swap_type: &SwapType, id: &str) -> Result<SwapState> {
	let state_dir = ensure_state_dir()?;
	let file_path = state_dir.join(swap_type).join(format!("{}.json", id));
	let json = fs::read_to_string(file_path)?;
	let state: SwapState = serde_json::from_str(&json)?;
	Ok(state)
}
