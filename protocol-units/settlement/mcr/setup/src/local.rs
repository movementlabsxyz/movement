use super::Setup;

use dot_movement::DotMovement;
use mcr_settlement_config::Config;

use alloy_primitives::hex;

use k256::ecdsa::SigningKey;
use tracing::info;

use std::future::Future;

/// The local setup strategy for MCR settlement
#[derive(Debug, Clone)]
pub struct Local {
	eth_rpc_port: u16,
	eth_ws_port: u16,
}

impl Local {
	/// Instantiates the local setup strategy with ports on localhost
	/// to configure for Ethernet RPC and WebSocket client access.
	pub fn new(eth_rpc_port: u16, eth_ws_port: u16) -> Self {
		Self { eth_rpc_port, eth_ws_port }
	}
}

impl Setup for Local {
	fn setup(
		&self,
		_dot_movement: &DotMovement,
		mut config: Config,
	) -> impl Future<Output = Result<Config, anyhow::Error>> + Send {
		async move {
			// Can't use rand 0.7 while k256 is on rand 0.6
			let mut rng = k256::elliptic_curve::rand_core::OsRng;

			info!("setting up MCR Ethereum client");

			if config.rpc_url.is_none() {
				config.rpc_url = Some(format!("http://localhost:{}", self.eth_rpc_port));
			}
			if config.ws_url.is_none() {
				config.ws_url = Some(format!("http://localhost:{}", self.eth_ws_port));
			}
			if config.signer_private_key.is_none() {
				let key = SigningKey::random(&mut rng);
				let key_bytes = key.to_bytes();
				let private_key = hex::encode(key_bytes.as_slice());
				config.signer_private_key = Some(private_key);
			}

			Ok(config)
		}
	}
}
