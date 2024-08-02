use alloy::{node_bindings::Anvil, providers::Provider};
use bridge_integration_tests::BridgeScaffold;
use ethereum_bridge::AtomicBridgeInitiator;

#[tokio::test]
async fn test_client_should_build_and_fetch_accounts() {
	let scaffold: BridgeScaffold = BridgeScaffold::new_only_eth().await;
	if scaffold.eth_client.is_none() {
		panic!("EthClient was not initialized properly.");
	}

	// Start Anvil with the fixed port
	let eth_client = scaffold.eth_client().expect("Failed to get EthClient");
	let anvil = Anvil::new().port(eth_client.rpc_port()).spawn();

	println!("Anvil running at `{}`", anvil.endpoint());

	let provider = scaffold.eth_client.unwrap().rpc_provider().clone();
	let accounts = provider.get_accounts().await.expect("Failed to get accounts");
}
