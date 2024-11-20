use crate::config::common::{
	default_celestia_rpc_connection_hostname, default_celestia_rpc_connection_port,
	default_celestia_rpc_connection_protocol, default_celestia_websocket_connection_hostname,
	default_celestia_websocket_connection_port, default_movement_da_light_node_connection_hostname,
	default_movement_da_light_node_connection_port, default_movement_da_light_node_listen_hostname,
	default_movement_da_light_node_listen_port,
};
use ecdsa::SigningKey;
use k256::Secp256k1;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DaSigners {
	pub private_key_hex: String,
	pub public_keys_hex: HashSet<String>,
}

/// The default da signing private key
pub fn default_da_signing_private_key() -> SigningKey<Secp256k1> {
	match std::env::var("DA_SIGNING_PRIVATE_KEY") {
		Ok(val) => {
			// decode from hex to bytes 32
			let hex_bytes = hex::decode(val).expect("Invalid hex string");

			// todo: maybe remove the unwrap and catch for a random signing key
			let signing_key_bytes: &[u8; 32] =
				hex_bytes.as_slice().try_into().expect("Slice with incorrect length");
			SigningKey::from_bytes(signing_key_bytes.into()).unwrap()
		}
		Err(std::env::VarError::NotPresent) => SigningKey::random(
			// rand_core
			&mut rand::rngs::OsRng,
		),
		Err(_) => panic!("Invalid DA_SIGNING_PRIVATE_KEY"),
	}
}

pub fn default_da_signers_sec1_keys() -> HashSet<String> {
	match std::env::var("DA_SIGNERS_SEC1_KEYS") {
		Ok(val) => val.split(',').map(|s| s.to_string()).collect(),
		Err(std::env::VarError::NotPresent) => HashSet::new(),
		Err(_) => panic!("Invalid DA_SIGNERS_SEC1_KEYS"),
	}
}

pub fn default_da_signers() -> DaSigners {
	let da_signer = default_da_signing_private_key();

	// always trust yourself
	let mut trusted_signers = HashSet::new();
	let sec1_hex = hex::encode(da_signer.verifying_key().to_sec1_bytes().to_vec());
	trusted_signers.insert(sec1_hex);

	// add the other specified signers
	let additional_signers = default_da_signers_sec1_keys();
	trusted_signers.extend(additional_signers);

	DaSigners {
		private_key_hex: hex::encode(da_signer.to_bytes().as_slice()),
		public_keys_hex: trusted_signers,
	}
}

#[cfg(test)]
pub mod signers_serialization_test {

	use super::*;

	#[test]
	fn test_signing_key() -> Result<(), anyhow::Error> {
		let signing_key = SigningKey::<Secp256k1>::random(
			// rand_core
			&mut rand::rngs::OsRng,
		);

		let signing_bytes = signing_key.to_bytes();

		let signing_key_fixed_bytes: &[u8; 32] =
			signing_bytes.as_slice().try_into().expect("Slice with incorrect length");

		let from_bytes =
			SigningKey::<Secp256k1>::from_bytes(&signing_key_fixed_bytes.clone().into())?;

		assert_eq!(signing_key, from_bytes);

		Ok(())
	}
}

/// The inner configuration for the local Celestia Appd Runner
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Config {
	/// The protocol for the Celestia RPC
	#[serde(default = "default_celestia_rpc_connection_protocol")]
	pub celestia_rpc_connection_protocol: String,

	/// The URL of the Celestia RPC
	#[serde(default = "default_celestia_rpc_connection_hostname")]
	pub celestia_rpc_connection_hostname: String,

	/// The port of the Celestia RPC
	#[serde(default = "default_celestia_rpc_connection_port")]
	pub celestia_rpc_connection_port: u16,

	/// The hostname of the Celestia Node websocket
	#[serde(default = "default_celestia_websocket_connection_hostname")]
	pub celestia_websocket_connection_hostname: String,

	/// The port of the Celestia Node websocket
	#[serde(default = "default_celestia_websocket_connection_port")]
	pub celestia_websocket_connection_port: u16,

	// FIXME: disentangle listen config for the light node service
	// from the connection config to connect to the same service?
	/// The hostname to listen on for the movement-celestia-da-light-node service
	#[serde(default = "default_movement_da_light_node_listen_hostname")]
	pub movement_da_light_node_listen_hostname: String,

	/// The port to listen on for the movement-celestia-da-light-node service
	#[serde(default = "default_movement_da_light_node_listen_port")]
	pub movement_da_light_node_listen_port: u16,

	/// The protocol for movement-celestia-da-light-node connection
	#[serde(default = "default_celestia_rpc_connection_protocol")]
	pub movement_da_light_node_connection_protocol: String,

	/// The hostname for movement-celestia-da-light-node connection
	#[serde(default = "default_movement_da_light_node_connection_hostname")]
	pub movement_da_light_node_connection_hostname: String,

	/// The port for movement-celestia-da-light-node connection
	#[serde(default = "default_movement_da_light_node_connection_port")]
	pub movement_da_light_node_connection_port: u16,

	/// The DA signers
	#[serde(default = "default_da_signers")]
	pub da_signers: DaSigners,
}

impl Default for Config {
	fn default() -> Self {
		Self {
			movement_da_light_node_connection_protocol: default_celestia_rpc_connection_protocol(),
			celestia_rpc_connection_protocol: default_celestia_rpc_connection_protocol(),
			celestia_rpc_connection_hostname: default_celestia_rpc_connection_hostname(),
			celestia_rpc_connection_port: default_celestia_rpc_connection_port(),
			celestia_websocket_connection_hostname: default_celestia_websocket_connection_hostname(
			),
			celestia_websocket_connection_port: default_celestia_websocket_connection_port(),
			movement_da_light_node_listen_hostname: default_movement_da_light_node_listen_hostname(
			),
			movement_da_light_node_listen_port: default_movement_da_light_node_listen_port(),
			movement_da_light_node_connection_hostname:
				default_movement_da_light_node_connection_hostname(),
			movement_da_light_node_connection_port: default_movement_da_light_node_connection_port(
			),
			da_signers: default_da_signers(),
		}
	}
}
