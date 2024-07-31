use ethereum_bridge::{Config as EthConfig, EthClient};
use movement_bridge::{Config as MovementConfig, MovementClient};

pub struct BridgeScaffold {
	pub eth_client: EthClient,
	pub movement_client: MovementClient,
}

impl BridgeScaffold {
	pub async fn new() -> Self {
		let eth_client = EthClient::new(EthConfig::default()).await;
		let movement_client = MovementClient::new(MovementConfig::default()).await;
		Self { eth_client, movement_client }
	}

	/// Compile and deploy a contract
	pub async fn deploy_contract(&self, contract_path: &str) -> Contract {
		// Compile the contract using solc
		let compiled = Solc::default().compile_source(contract_path).expect("Failed to compile");

		let (abi, bytecode) = compiled
			.get(&compiled.contracts[0].id)
			.expect("Contract not found")
			.into_parts();

		// Deploy the contract
		let factory = ContractFactory::new(abi, bytecode, self.client.clone());
		let contract = factory
			.deploy(())
			.expect("Failed to deploy contract")
			.send()
			.await
			.expect("Contract deployment failed");

		contract
	}
}
