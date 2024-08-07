use alloy::{
	node_bindings::{Anvil, AnvilInstance},
	primitives::{address, keccak256, Address},
	providers::{Provider, WalletProvider},
	signers::{
		k256::ecdsa::{SigningKey, VerifyingKey},
		local::{LocalSigner, PrivateKeySigner},
	},
	sol,
};
use alloy_network::EthereumWallet;
use aptos_sdk::types::LocalAccount;
use bridge_integration_tests::TestHarness;
use bridge_shared::{
	bridge_contracts::BridgeContractInitiator,
	types::{Amount, BridgeTransferId, HashLock, InitiatorAddress, RecipientAddress, TimeLock},
};
use ethereum_bridge::{types::EthAddress, AtomicBridgeInitiator};
use rand::SeedableRng;

#[tokio::test]
async fn test_client_should_build_and_fetch_accounts() {
	let scaffold: TestHarness = TestHarness::new_only_eth().await;
	if scaffold.eth_client.is_none() {
		panic!("EthClient was not initialized properly.");
	}

	// Start Anvil with the fixed port
	let eth_client = scaffold.eth_client().expect("Failed to get EthClient");
	let anvil = Anvil::new().port(eth_client.rpc_port()).spawn();

	println!("Anvil running at `{}`", anvil.endpoint());

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
async fn test_client_should_deploy_initiator_contract() {
	let mut harness: TestHarness = TestHarness::new_only_eth().await;
	let anvil = Anvil::new().port(harness.rpc_port()).spawn();
	let signer_address = harness.set_eth_signer(anvil.keys()[0].clone());

	let initiator_address = harness.deploy_initiator_contract().await;
	let expected_address = address!("5fbdb2315678afecb367f032d93f642f64180aa3");

	assert_eq!(initiator_address, expected_address);
}

#[tokio::test]
async fn test_client_should_successfully_call_initialize() {
	let mut harness: TestHarness = TestHarness::new_only_eth().await;
	let anvil = Anvil::new().port(harness.rpc_port()).spawn();

	let (harness, anvil) = deploy_init_contracts(harness, &anvil).await;
	let signer_address = harness.eth_signer_address();
}

#[tokio::test]
async fn test_client_should_successfully_call_initiate_transfer() {
	let mut harness: TestHarness = TestHarness::new_only_eth().await;
	let anvil = Anvil::new().port(harness.rpc_port()).spawn();
	println!("Anvil running at `{}`", anvil.endpoint());

	//set a funded signer
	harness.set_eth_signer(anvil.keys()[0].clone());

	harness.deploy_init_contracts().await;
	let chain_id = anvil.chain_id();
	println!("chain_id: {:?}", chain_id);
	// harness
	// 	.eth_client()
	// 	.expect("Could not get client")
	// 	.get_weth_initiator_contract()
	// 	.await
	// 	.expect("could not fetch weth contract");

	// Gen an aptos account
	let mut rng = ::rand::rngs::StdRng::from_seed([3u8; 32]);
	let movement_recipient = LocalAccount::generate(&mut rng);
	println!("movement_recipient: {:?}", movement_recipient);
	let recipient_bytes: Vec<u8> = movement_recipient.public_key().to_bytes().to_vec();
	println!("recipient_bytes length: {:?}", recipient_bytes.len());

	let secret = "secret".to_string();
	let hash_lock: [u8; 32] = keccak256(secret.as_bytes()).into();

	let signer_address = harness.eth_signer_address();
	println!("signer_address: {:?}", signer_address);

	//sleep for a bit to allow the contract to be mined
	tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

	let _ = harness
		.eth_client_mut()
		.expect("Failed to get EthClient")
		.initiate_bridge_transfer(
			InitiatorAddress(EthAddress(signer_address)),
			RecipientAddress(recipient_bytes),
			HashLock(hash_lock),
			TimeLock(100),
			Amount(1000), // Eth
		)
		.await
		.expect("Failed to initiate bridge transfer");

	//@TODO: Here we should assert on the event emitted by the contract
}

// #[tokio::test]
// async fn test_client_should_successfully_get_bridge_transfer_id() {
// 	let scaffold: TestHarness = TestHarness::new_only_eth().await;
// 	if scaffold.eth_client.is_none() {
// 		panic!("EthClient was not initialized properly.");
// 	}
//
// 	let mut eth_client = scaffold.eth_client().expect("Failed to get EthClient");
// 	let anvil = Anvil::new().port(eth_client.rpc_port()).spawn();
// 	println!("Anvil running at `{}`", anvil.endpoint());
//
// 	// set funded signer
// 	let signer = anvil.keys()[0].clone();
// 	let mut provider = scaffold.eth_client.unwrap().rpc_provider().clone();
// 	let mut wallet: &mut EthereumWallet = provider.wallet_mut();
// 	wallet.register_default_signer(LocalSigner::from(signer));
//
// 	let contract = AtomicBridgeInitiator::deploy(&provider)
// 		.await
// 		.expect("Failed to deploy contract");
//
// 	let expected_address = address!("5fbdb2315678afecb367f032d93f642f64180aa3");
// 	assert_eq!(contract.address(), &expected_address);
//
// 	//some data to set for the recipient.
// 	let recipient = address!("70997970c51812dc3a010c7d01b50e0d17dc79c8");
// 	let recipient_bytes: Vec<u8> = recipient.to_string().as_bytes().to_vec();
//
// 	let secret = "secret".to_string();
// 	let hash_lock = keccak256(secret.as_bytes());
// 	let hash_lock: [u8; 32] = hash_lock.into();
//
// 	let _ = eth_client
// 		.initiate_bridge_transfer(
// 			InitiatorAddress(EthAddress(expected_address)),
// 			RecipientAddress(recipient_bytes),
// 			HashLock(hash_lock),
// 			TimeLock(1000),
// 			Amount(42),
// 		)
// 		.await
// 		.expect("Failed to initiate bridge transfer");
//
// 	let bridge_transfer_details = eth_client
// 		.get_bridge_transfer_details(BridgeTransferId([0u8; 32]))
// 		.await
// 		.expect("Failed to get bridge transfer details");
// }

// #[tokio::test]
// async fn test_client_should_successfully_complete_transfer() {
// 	let scaffold: TestHarness = TestHarness::new_only_eth().await;
// 	if scaffold.eth_client.is_none() {
// 		panic!("EthClient was not initialized properly.");
// 	}
//
// 	let mut eth_client = scaffold.eth_client().expect("Failed to get EthClient");
// 	let anvil = Anvil::new().port(eth_client.rpc_port()).spawn();
// 	println!("Anvil running at `{}`", anvil.endpoint());
//
// 	// set funded signer
// 	let signer = anvil.keys()[0].clone();
// 	let mut provider = scaffold.eth_client.unwrap().rpc_provider().clone();
// 	let mut wallet: &mut EthereumWallet = provider.wallet_mut();
// 	wallet.register_default_signer(LocalSigner::from(signer));
//
// 	let contract = AtomicBridgeInitiator::deploy(&provider)
// 		.await
// 		.expect("Failed to deploy contract");
//
// 	let expected_address = address!("5fbdb2315678afecb367f032d93f642f64180aa3");
// 	assert_eq!(contract.address(), &expected_address);
//
// 	//some data to set for the recipient.
// 	let recipient = address!("70997970c51812dc3a010c7d01b50e0d17dc79c8");
// 	let recipient_bytes: Vec<u8> = recipient.to_string().as_bytes().to_vec();
//
// 	let secret = "secret".to_string();
// 	let hash_lock = keccak256(secret.as_bytes());
// 	let hash_lock: [u8; 32] = hash_lock.into();
//
// 	let _ = eth_client
// 		.initiate_bridge_transfer(
// 			InitiatorAddress(EthAddress(expected_address)),
// 			RecipientAddress(recipient_bytes),
// 			HashLock(hash_lock),
// 			TimeLock(1000),
// 			Amount(42),
// 		)
// 		.await
// 		.expect("Failed to initiate bridge transfer");
//
// 	let bridge_transfer_details = eth_client
// 		.get_bridge_transfer_details(BridgeTransferId([0u8; 32]))
// 		.await
// 		.expect("Failed to get bridge transfer details");
//
// 	let secret = "secret".to_string();
// 	let hash_lock = keccak256(secret.as_bytes());
// 	let hash_lock: [u8; 32] = hash_lock.into();
//
// 	// let _ = eth_client
// 	// 	.complete_bridge_transfer()
// 	// 	.await
// 	// 	.expect
// }
//

async fn deploy_init_contracts(
	mut harness: TestHarness,
	anvil: &AnvilInstance,
) -> (TestHarness, &AnvilInstance) {
	let signer_address = harness.set_eth_signer(anvil.keys()[0].to_owned());
	let provider = harness.provider();

	let initiator_address = harness.deploy_initiator_contract().await;
	println!("deployed initiator contract at: {:?}", initiator_address);

	let weth_address = harness.deploy_weth_contract().await;
	println!("deployed weth contract at: {:?}", weth_address);

	harness
		.eth_client()
		.expect("Failed to get EthClient")
		.initialize_initiator_contract(EthAddress(weth_address), EthAddress(signer_address))
		.await
		.expect("Failed to initialize contract");

	println!("initialized initiator contract");

	(harness, anvil)
}
