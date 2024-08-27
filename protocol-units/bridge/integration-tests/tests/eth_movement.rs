use alloy::{
	node_bindings::Anvil,
	primitives::{address, keccak256, U256},
	providers::Provider,
	signers::local::yubihsm::ecdsa::Signer,
};
use anyhow::Context;
use anyhow::Result;

use aptos_sdk::{
	coin_client::CoinClient,
	types::LocalAccount,
};
use bridge_integration_tests::TestHarness;
use bridge_shared::{
	bridge_contracts::{BridgeContractCounterparty, BridgeContractInitiator}, bridge_monitoring::BridgeContractInitiatorEvent, types::{Amount, BridgeTransferId, HashLock, HashLockPreImage, InitiatorAddress, RecipientAddress, TimeLock}
};

use ethereum_bridge::{
	types::EthAddress,
};

use movement_bridge::utils::MovementAddress;
use rand;
use tokio;
use futures::{channel::mpsc::{self, UnboundedReceiver}, StreamExt};

use aptos_types::account_address::AccountAddress;
use tracing;
use tracing_subscriber;

#[tokio::test]
async fn test_movement_client_build_and_fund_accounts() -> Result<(), anyhow::Error> {
	let (scaffold, mut child) = TestHarness::new_with_movement().await;
	let movement_client = scaffold.movement_client().expect("Failed to get MovementClient");
// 
	let rest_client = movement_client.rest_client();
	let coin_client = CoinClient::new(&rest_client);
	let faucet_client = movement_client.faucet_client().expect("Failed to get // FaucetClient");
	let alice = LocalAccount::generate(&mut rand::rngs::OsRng);
	let bob = LocalAccount::generate(&mut rand::rngs::OsRng);

	println!("\n=== Addresses ===");
	println!("Alice: {}", alice.address().to_hex_literal());
	println!("Bob: {}", bob.address().to_hex_literal());
	let faucet_client = faucet_client.write().unwrap();
	faucet_client
		.fund(alice.address(), 100_000_000)
		.await
		.context("Failed to fund Alice's account")?;
	faucet_client
		.create_account(bob.address())
		.await
		.context("Failed to fund Bob's account")?;

	println!("\n=== Initial Balances ===");
	println!(
		"Alice: {:?}",
		coin_client
			.get_account_balance(&alice.address())
			.await
			.context("Failed to get Alice's account balance")?
	);
	println!(
		"Bob: {:?}",
		coin_client
			.get_account_balance(&bob.address())
			.await
			.context("Failed to get Bob's account balance")?
	);
	child.kill().await.context("Failed to kill the child process")?;

	Ok(())
}

#[tokio::test]
async fn test_movement_client_should_publish_package() -> Result<(), anyhow::Error> {
	let _ = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .try_init();
	
	let (mut harness, mut child) = TestHarness::new_with_movement().await;
	{ let movement_client = harness.movement_client_mut().expect("Failed to get MovementClient");

	let _ = movement_client.publish_for_test();
	}

	child.kill().await.context("Failed to kill the child process")?;
	
	Ok(())
}

#[tokio::test]
#[ignore]
async fn test_movement_client_should_successfully_call_lock() -> Result<(), anyhow::Error> {

	let _ = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .try_init();
	
	let (mut harness, mut child) = TestHarness::new_with_movement().await;
	{ let movement_client = harness.movement_client_mut().expect("Failed to get MovementClient");

	let _ = movement_client.publish_for_test();
	}
	
        let initiator = b"0x123".to_vec(); //In real world this would be an ethereum address
        let recipient:MovementAddress = MovementAddress(AccountAddress::new(*b"0x00000000000000000000000000face"));    
        let bridge_transfer_id = *b"00000000000000000000000transfer1";
        //let pre_image = b"secret".to_vec();
        let hash_lock = *keccak256(b"secret".to_vec()); 
        let time_lock = 3600;
        let amount = 100;
	
	harness
	.movement_client_mut()
	.expect("Failed to get MovmentClient")
	.lock_bridge_transfer_assets(
		BridgeTransferId(bridge_transfer_id),		
		HashLock(hash_lock),
		TimeLock(time_lock),
		InitiatorAddress(initiator),
		RecipientAddress(recipient),
		Amount(amount), // Eth
	)
	.await
	.expect("Failed to complete bridge transfer");

	harness
	.movement_client_mut()
	.expect("Failed to get MovmentClient")
	.complete_bridge_transfer(
		BridgeTransferId(bridge_transfer_id),
		HashLockPreImage(b"secret".to_vec()),
	)
	.await
	.expect("Failed to complete bridge transfer");

	child.kill().await.context("Failed to kill the child process")?;
	//let _ = child.wait().await.expect("Failed to wait on process termination");
	Ok(())
}

#[tokio::test]
async fn test_movement_client_complete_bridge_transfer() -> Result<(), anyhow::Error> {
	
	Ok(())
}

#[tokio::test]
async fn test_movement_client_abort_bridge_transfer() -> Result<(), anyhow::Error> {
	
	Ok(())
}

#[tokio::test]
async fn test_eth_client_should_build_and_fetch_accounts() {
	let scaffold: TestHarness = TestHarness::new_only_eth().await;

	let eth_client = scaffold.eth_client().expect("Failed to get EthClient");
	let _anvil = Anvil::new().port(eth_client.rpc_port()).spawn();

	let expected_accounts = vec![
		address!("f39fd6e51aad88f6f4ce6ab8827279cfffb92266"),
		address!("70997970c51812dc3a010c7d01b50e0d17dc79c8"),
		address!("3c44cdddb6a900fa2b585dd299e03d12fa4293bc"),
		address!("90f79bf6eb2c4f870365e785982e1f101e93b906"),
		address!("15d34aaf54267db7d7c367839aaf71a00a2c6a65"),
		address!("9965507d1a55bcc2695c58ba16fb37d819b0a4dc"),
		address!("976ea74026e726554db657fa54763abd0c3a0aa9"),
		address!("14dc79964da2c08b23698b3d3cc7ca32193d9955"),
		address!("23618e81e3f5cdf7f54c3d65f7fbc0abf5b21e8f"),
		address!("a0ee7a142d267c1f36714e4a8f75612f20a79720"),
	];

	let provider = scaffold.eth_client.unwrap().rpc_provider().clone();
	println!("provider: {:?}", provider);
	let accounts = provider.get_accounts().await.expect("Failed to get accounts");
	assert_eq!(accounts.len(), expected_accounts.len());

	for (account, expected) in accounts.iter().zip(expected_accounts.iter()) {
		assert_eq!(account, expected);
	}
}

#[tokio::test]
async fn test_eth_client_should_deploy_initiator_contract() {
	let mut harness: TestHarness = TestHarness::new_only_eth().await;
	let anvil = Anvil::new().port(harness.rpc_port()).spawn();

	let _ = harness.set_eth_signer(anvil.keys()[0].clone());

	let initiator_address = harness.deploy_initiator_contract().await;
	let expected_address = address!("5fbdb2315678afecb367f032d93f642f64180aa3");

	assert_eq!(initiator_address, expected_address);
}

#[tokio::test]
async fn test_eth_client_should_successfully_call_initialize() {
	let mut harness: TestHarness = TestHarness::new_only_eth().await;
	let anvil = Anvil::new().port(harness.rpc_port()).spawn();

	let _ = harness.set_eth_signer(anvil.keys()[0].clone());
	harness.deploy_init_contracts().await;
}

#[tokio::test]
async fn test_eth_client_should_successfully_call_initiate_transfer() {
	let mut harness: TestHarness = TestHarness::new_only_eth().await;
	let anvil = Anvil::new().port(harness.rpc_port()).spawn();

	let signer_address = harness.set_eth_signer(anvil.keys()[0].clone());

	harness.deploy_init_contracts().await;

	let recipient = harness.gen_aptos_account();
	let hash_lock: [u8; 32] = keccak256("secret".to_string().as_bytes()).into();

	harness
		.eth_client_mut()
		.expect("Failed to get EthClient")
		.initiate_bridge_transfer(
			InitiatorAddress(EthAddress(signer_address)),
			RecipientAddress(recipient),
			HashLock(hash_lock),
			TimeLock(100),
			// value has to be > 0
			Amount(AssetType::EthAndWeth((1, 0))), // Eth
		)
		.await
		.expect("Failed to initiate bridge transfer");
}

#[tokio::test]
async fn test_client_should_successfully_call_initiate_transfer_only_weth() {
	let mut harness: TestHarness = TestHarness::new_only_eth().await;
	let anvil = Anvil::new().port(harness.rpc_port()).spawn();

	let signer_address = harness.set_eth_signer(anvil.keys()[0].clone());
	let matching_signer_address = harness.eth_signer_address();

	assert_eq!(signer_address, matching_signer_address, "Signer address mismatch");

	harness.deploy_init_contracts().await;

	let recipient = harness.gen_aptos_account();
	let hash_lock: [u8; 32] = keccak256("secret".to_string().as_bytes()).into();
	harness
		.deposit_weth_and_approve(
			InitiatorAddress(EthAddress(signer_address)),
			Amount(AssetType::EthAndWeth((0, 1))),
		)
		.await
		.expect("Failed to deposit WETH");
	harness
		.initiate_bridge_transfer(
			InitiatorAddress(EthAddress(signer_address)),
			RecipientAddress(recipient),
			HashLock(hash_lock),
			TimeLock(100),
			Amount(AssetType::EthAndWeth((0, 1))),
		)
		.await
		.expect("Failed to initiate bridge transfer");
}

#[tokio::test]
async fn test_client_should_successfully_call_initiate_transfer_eth_and_weth() {
	let mut harness: TestHarness = TestHarness::new_only_eth().await;
	let anvil = Anvil::new().port(harness.rpc_port()).spawn();

	let signer_address = harness.set_eth_signer(anvil.keys()[0].clone());
	let matching_signer_address = harness.eth_signer_address();

	assert_eq!(signer_address, matching_signer_address, "Signer address mismatch");

	harness.deploy_init_contracts().await;

	let recipient = harness.gen_aptos_account();
	let hash_lock: [u8; 32] = keccak256("secret".to_string().as_bytes()).into();
	harness
		.deposit_weth_and_approve(
			InitiatorAddress(EthAddress(signer_address)),
			Amount(AssetType::EthAndWeth((0, 1))),
		)
		.await
		.expect("Failed to deposit WETH");
	harness
		.initiate_bridge_transfer(
			InitiatorAddress(EthAddress(signer_address)),
			RecipientAddress(recipient),
			HashLock(hash_lock),
			TimeLock(100),
			Amount(AssetType::EthAndWeth((1, 1))),
		)
		.await
		.expect("Failed to initiate bridge transfer");
}

#[tokio::test]
#[ignore] // To be tested after this is merged in https://github.com/movementlabsxyz/movement/pull/209
async fn test_client_should_successfully_get_bridge_transfer_id() {
	let mut harness: TestHarness = TestHarness::new_only_eth().await;
	let anvil = Anvil::new().port(harness.rpc_port()).spawn();

	let signer_address = harness.set_eth_signer(anvil.keys()[0].clone());
	harness.deploy_init_contracts().await;

	let recipient = harness.gen_aptos_account();
	let hash_lock: [u8; 32] = keccak256("secret".to_string().as_bytes()).into();

	harness
		.eth_client_mut()
		.expect("Failed to get EthClient")
		.initiate_bridge_transfer(
			InitiatorAddress(EthAddress(signer_address)),
			RecipientAddress(recipient),
			HashLock(hash_lock),
			TimeLock(100),
			Amount(AssetType::EthAndWeth((1000, 0))), // Eth
		)
		.await
		.expect("Failed to initiate bridge transfer");

	//TODO: Here call get details with the captured event
}

#[tokio::test]
#[ignore] // To be tested after this is merged in https://github.com/movementlabsxyz/movement/pull/209
async fn test_eth_client_should_successfully_complete_transfer() {
	let mut harness: TestHarness = TestHarness::new_only_eth().await;
	let anvil = Anvil::new().port(harness.rpc_port()).spawn();

	let signer_address = harness.set_eth_signer(anvil.keys()[0].clone());
	harness.deploy_init_contracts().await;

	let recipient = address!("70997970c51812dc3a010c7d01b50e0d17dc79c8");
	let recipient_bytes: Vec<u8> = recipient.to_string().as_bytes().to_vec();

	let secret = "secret".to_string();
	let hash_lock = keccak256(secret.as_bytes());
	let hash_lock: [u8; 32] = hash_lock.into();

	let _ = harness
		.eth_client_mut()
		.expect("Failed to get EthClient")
		.initiate_bridge_transfer(
			InitiatorAddress(EthAddress(signer_address)),
			RecipientAddress(recipient_bytes),
			HashLock(hash_lock),
			TimeLock(1000),
			Amount(AssetType::EthAndWeth((42, 0))),
		)
		.await
		.expect("Failed to initiate bridge transfer");

	//TODO: Here call complete with the id captured from the event
}
