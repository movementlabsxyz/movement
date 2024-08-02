use alloy::{
	node_bindings::Anvil,
	primitives::U256,
	providers::{Provider, ProviderBuilder},
	signers::local::PrivateKeySigner,
};
use alloy_network::EthereumWallet;
use alloy_sol_types::sol;
use anyhow::{Error, Result};
use ethereum_bridge::{Config as EthConfig, EthClient};
use movement_bridge::{Config as MovementConfig, MovementClient};

sol!(
	#[allow(missing_docs)]
	#[sol(rpc)]
	AtomicBridgeInitiator,
	"../chains/ethereum/abis/AtomicBridgeInitiator.json"
);

sol!(
	#[allow(missing_docs)]
	#[sol(rpc)]
	AtomicBridgeCounterparty,
	"../chains/ethereum/abis/AtomicBridgeCounterparty.json"
);

pub struct BridgeScaffold {
	pub eth_client: Option<EthClient>,
	pub movement_client: Option<MovementClient>,
}

impl BridgeScaffold {
	pub async fn new_only_eth() -> Self {
		let eth_client = EthClient::new(EthConfig::build_for_test())
			.await
			.expect("Failed to create EthClient");
		Self { eth_client: Some(eth_client), movement_client: None }
	}

	/// Compile and deploy a contract
	pub async fn deploy_contract(&self) -> Result<()> {
		let eth_client = self.eth_client.as_ref().expect("EthClient not found");
		// Start Anvil with the fixed port
		let anvil = Anvil::new().port(eth_client.rpc_port()).spawn();

		// Set up signer from the first default Anvil account (Alice).
		let signer: PrivateKeySigner = anvil.keys()[0].clone().into();
		let wallet = EthereumWallet::from(signer);

		// Create a provider with the wallet.
		let rpc_url = anvil.endpoint().parse()?;
		let provider = ProviderBuilder::new()
			.with_recommended_fillers()
			.wallet(wallet)
			.on_http(rpc_url);

		println!("Anvil running at `{}`", anvil.endpoint());

		// Deploy the `Counter` contract.
		let contract = AtomicBridgeInitiator::deploy(&provider).await?;

		println!("Deployed contract at address: {}", contract.address());
		Ok(())
	}
}
