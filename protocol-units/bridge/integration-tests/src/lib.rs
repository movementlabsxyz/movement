use alloy::{
	node_bindings::Anvil,
	primitives::U256,
	providers::{Provider, ProviderBuilder},
	signers::local::PrivateKeySigner,
};
use alloy_network::EthereumWallet;
use alloy_sol_types::sol;
use anyhow::{Error, Result};
use ethereum_bridge::{
	AtomicBridgeCounterparty, AtomicBridgeInitiator, Config as EthConfig, EthClient,
};
use movement_bridge::{Config as MovementConfig, MovementClient};

pub struct TestScaffold {
	pub eth_client: Option<EthClient>,
	pub movement_client: Option<MovementClient>,
}

impl TestScaffold {
	pub async fn new_only_eth() -> Self {
		let eth_client = EthClient::new(EthConfig::build_for_test())
			.await
			.expect("Failed to create EthClient");
		Self { eth_client: Some(eth_client), movement_client: None }
	}

	pub fn eth_client(&self) -> Result<&EthClient> {
		self.eth_client.as_ref().ok_or(Error::msg("EthClient not found"))
	}
}
