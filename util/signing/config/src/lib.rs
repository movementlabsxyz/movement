//! This crate provides configuration parameters for signing KeyManager
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub enum KeyProvider {
	#[default]
	LOCALETH,
	LOCALMVT,
	AWSKMS,
	VAULT,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct KeyDefinition {
	#[serde(default)]
	pub name: String,
	#[serde(default)]
	pub provider: KeyProvider,
	#[serde(default)]
	pub id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Config {
	#[serde(default)]
	pub key_list: Vec<KeyDefinition>,
}
