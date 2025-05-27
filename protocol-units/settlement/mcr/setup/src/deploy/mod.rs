use alloy::providers::Provider;
use alloy::providers::ProviderBuilder;
use alloy::signers::Signer;
use alloy_network::EthereumWallet;
use alloy_network::TransactionBuilder;
use alloy_primitives::U256;
use anyhow::anyhow;
use anyhow::Context;
use commander::run_command;
use dot_movement::DotMovement;
use mcr_settlement_client::eth_client::MCR;
use mcr_settlement_config::{common, Config};
use movement_signer::cryptography::secp256k1::Secp256k1;
use movement_signer_aws_kms::hsm::AwsKms;
use movement_signer_loader::identifiers::SignerIdentifier;
use movement_signing_eth::HsmSigner;
use serde_json::Value;
use std::str::FromStr;
use tracing::info;

/// The local setup strategy for MCR settlement
#[derive(Debug, Clone)]
pub struct Deploy {}

impl Deploy {
	/// Instantiates the local setup strategy with ports on localhost
	/// to configure for Ethernet RPC and WebSocket client access.
	pub fn new() -> Self {
		Self {}
	}
}

impl Default for Deploy {
	fn default() -> Self {
		Deploy::new()
	}
}

impl Deploy {
	pub async fn setup(
		&self,
		_dot_movement: &DotMovement,
		mut config: Config,
		deploy: &common::deploy::Config,
	) -> Result<Config, anyhow::Error> {
		// enforce config.deploy = deploy
		Ok(config)
	}
}
