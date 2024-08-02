use alloy::providers::Provider;
use bridge_integration_tests::BridgeScaffold;

#[tokio::test]
async fn test_eth_to_movement() {
	let scaffold: BridgeScaffold = BridgeScaffold::new_only_eth().await;
	if scaffold.eth_client.is_none() {
		panic!("EthClient was not initialized properly.");
	}

	let provider = scaffold.eth_client.unwrap().rpc_provider.clone();
	println!("Provider: {:?}", provider);
	let accounts = provider.get_accounts().await.expect("Failed to get accounts");
}
