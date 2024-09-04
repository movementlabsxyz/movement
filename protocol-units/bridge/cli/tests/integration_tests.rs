use alloy::{
	node_bindings::Anvil,
	primitives::{Address, FixedBytes},
	providers::ProviderBuilder,
};
use bridge_cli::{
	clap::eth_to_movement::{self, EthSharedArgs},
	eth_to_moveth,
};
use ethereum_bridge::types::{AtomicBridgeInitiator, EthAddress};
use movement_bridge::utils::MovementAddress;
use std::str::FromStr;
use url::Url;

#[tokio::test]
async fn test_swap() -> eyre::Result<()> {
	// Start Anvil instance
	let anvil = Anvil::new().try_spawn()?;
	let rpc_url = anvil.endpoint().parse()?;
	let provider = ProviderBuilder::new().on_http(rpc_url);

	// Deploy contracts
	let wallet = anvil.keys()[0].clone();

	let initiator_contract = AtomicBridgeInitiator::deploy(provider).await?;

	// Set up EthSharedArgs
	let eth_shared_args: EthSharedArgs = EthSharedArgs {
		eth_private_key: wallet.into(),
		eth_rpc_url: Url::parse(&anvil.endpoint()).unwrap(),
		eth_ws_url: Url::parse(&anvil.endpoint().replace("http", "ws")).unwrap(),
		eth_initiator_contract: EthAddress(*initiator_contract.address()),
		eth_counterparty_contract: EthAddress(Address::ZERO), // Not needed for this test
		eth_weth_contract: EthAddress(Address::ZERO), // Not needed for this test
		eth_gas_limit: 3000000,
	};

	// Prepare swap parameters
	let recipient: MovementAddress =
		"0x000000000000000000000000000000000000000000000000000000000000000A"
			.parse()
			.unwrap();
	let amount = 1000000000000000000u64; // 1 ETH in wei

	// Execute the swap
	let result = eth_to_moveth::execute(&eth_to_movement::Commands::ToMovement {
		args: eth_shared_args,
		recipient: From::from(recipient),
		amount,
	})
	.await;

	assert!(result.is_ok(), "Swap initiation failed: {:?}", result.err());

	// Check on the contract if we have a bridge transfer initiated
	let bridge_transfer_id = FixedBytes::from_str("0x1234567890123456789012345678901234567890")?;
	initiator_contract.bridgeTransfers(bridge_transfer_id).call().await?;

	Ok(())
}
