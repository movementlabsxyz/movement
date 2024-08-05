use alloy::{
	node_bindings::Anvil,
	primitives::{address, keccak256, Address},
	providers::{Provider, WalletProvider},
	signers::{
		k256::ecdsa::SigningKey,
		local::{LocalSigner, PrivateKeySigner},
	},
	sol,
};
use alloy_network::EthereumWallet;
use bridge_integration_tests::TestScaffold;
use bridge_shared::{
	bridge_contracts::BridgeContractInitiator,
	types::{Amount, BridgeTransferId, HashLock, InitiatorAddress, RecipientAddress, TimeLock},
};
use ethereum_bridge::{types::EthAddress, AtomicBridgeInitiator};

alloy::sol!(
	#[allow(missing_docs)]
	#[sol(rpc)]
	WETH9,
	"../chains/ethereum/abis/WETH9.json"
);

#[tokio::test]
async fn test_client_should_build_and_fetch_accounts() {
	let scaffold: TestScaffold = TestScaffold::new_only_eth().await;
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
async fn test_client_should_deploy_contract() {
	let scaffold: TestScaffold = TestScaffold::new_only_eth().await;
	if scaffold.eth_client.is_none() {
		panic!("EthClient was not initialized properly.");
	}

	let eth_client = scaffold.eth_client().expect("Failed to get EthClient");
	let anvil = Anvil::new().port(eth_client.rpc_port()).spawn();
	println!("Anvil running at `{}`", anvil.endpoint());

	let signer = anvil.keys()[0].clone();
	let mut provider = scaffold.eth_client.unwrap().rpc_provider().clone();
	let mut wallet: &mut EthereumWallet = provider.wallet_mut();
	wallet.register_default_signer(LocalSigner::from(signer));
	let contract = AtomicBridgeInitiator::deploy(&provider)
		.await
		.expect("Failed to deploy contract");

	let expected_address = address!("5fbdb2315678afecb367f032d93f642f64180aa3");
	assert_eq!(contract.address(), &expected_address);
}

#[tokio::test]
async fn test_client_should_successfully_call_initialize() {
	let scaffold: TestScaffold = TestScaffold::new_only_eth().await;
	if scaffold.eth_client.is_none() {
		panic!("EthClient was not initialized properly.");
	}

	let eth_client = scaffold.eth_client().expect("Failed to get EthClient");
	let anvil = Anvil::new().port(eth_client.rpc_port()).spawn();
	println!("Anvil running at `{}`", anvil.endpoint());

	let signer = anvil.keys()[0].clone();
	let mut provider = scaffold.eth_client.unwrap().rpc_provider().clone();
	let mut wallet: &mut EthereumWallet = provider.wallet_mut();
	wallet.register_default_signer(LocalSigner::from(signer));
	let _ = AtomicBridgeInitiator::deploy(&provider)
		.await
		.expect("Failed to deploy contract");

	let weth_contract = WETH9::deploy(&provider).await.expect("Failed to deploy contract");
	assert_eq!(weth_contract.address(), &address!("e7f1725e7734ce288f8367e1bb143e90bb3f0512"));
}

#[tokio::test]
async fn test_client_should_successfully_call_initiate_transfer() {
	let scaffold: TestScaffold = TestScaffold::new_only_eth().await;
	if scaffold.eth_client.is_none() {
		panic!("EthClient was not initialized properly.");
	}

	let mut eth_client = scaffold.eth_client().expect("Failed to get EthClient");
	let anvil = Anvil::new().port(eth_client.rpc_port()).spawn();
	println!("Anvil running at `{}`", anvil.endpoint());

	// set funded signer
	let signer = anvil.keys()[0].clone();
	let mut provider = scaffold.eth_client.unwrap().rpc_provider().clone();
	let mut wallet: &mut EthereumWallet = provider.wallet_mut();
	wallet.register_default_signer(LocalSigner::from(signer));

	let contract = AtomicBridgeInitiator::deploy(&provider)
		.await
		.expect("Failed to deploy contract");

	let expected_address = address!("5fbdb2315678afecb367f032d93f642f64180aa3");
	assert_eq!(contract.address(), &expected_address);
	eth_client.set_initiator_contract(contract.address().clone());

	//some data to set for the recipient.
	let recipient = address!("70997970c51812dc3a010c7d01b50e0d17dc79c8");
	let recipient_bytes: Vec<u8> = recipient.into_word().to_vec();

	let secret = "secret".to_string();
	let hash_lock = keccak256(secret.as_bytes());
	let hash_lock: [u8; 32] = hash_lock.into();

	let _ = eth_client
		.initiate_bridge_transfer(
			InitiatorAddress(EthAddress(expected_address)),
			RecipientAddress(recipient_bytes),
			HashLock(hash_lock),
			TimeLock(100_000_000),
			Amount(42),
		)
		.await
		.expect("Failed to initiate bridge transfer");

	//@TODO: Here we should assert on the event emitted by the contract
}

#[tokio::test]
async fn test_client_should_successfully_get_bridge_transfer_id() {
	let scaffold: TestScaffold = TestScaffold::new_only_eth().await;
	if scaffold.eth_client.is_none() {
		panic!("EthClient was not initialized properly.");
	}

	let mut eth_client = scaffold.eth_client().expect("Failed to get EthClient");
	let anvil = Anvil::new().port(eth_client.rpc_port()).spawn();
	println!("Anvil running at `{}`", anvil.endpoint());

	// set funded signer
	let signer = anvil.keys()[0].clone();
	let mut provider = scaffold.eth_client.unwrap().rpc_provider().clone();
	let mut wallet: &mut EthereumWallet = provider.wallet_mut();
	wallet.register_default_signer(LocalSigner::from(signer));

	let contract = AtomicBridgeInitiator::deploy(&provider)
		.await
		.expect("Failed to deploy contract");

	let expected_address = address!("5fbdb2315678afecb367f032d93f642f64180aa3");
	assert_eq!(contract.address(), &expected_address);

	//some data to set for the recipient.
	let recipient = address!("70997970c51812dc3a010c7d01b50e0d17dc79c8");
	let recipient_bytes: Vec<u8> = recipient.to_string().as_bytes().to_vec();

	let secret = "secret".to_string();
	let hash_lock = keccak256(secret.as_bytes());
	let hash_lock: [u8; 32] = hash_lock.into();

	let _ = eth_client
		.initiate_bridge_transfer(
			InitiatorAddress(EthAddress(expected_address)),
			RecipientAddress(recipient_bytes),
			HashLock(hash_lock),
			TimeLock(1000),
			Amount(42),
		)
		.await
		.expect("Failed to initiate bridge transfer");

	let bridge_transfer_details = eth_client
		.get_bridge_transfer_details(BridgeTransferId([0u8; 32]))
		.await
		.expect("Failed to get bridge transfer details");
}

#[tokio::test]
async fn test_client_should_successfully_complete_transfer() {
	let scaffold: TestScaffold = TestScaffold::new_only_eth().await;
	if scaffold.eth_client.is_none() {
		panic!("EthClient was not initialized properly.");
	}

	let mut eth_client = scaffold.eth_client().expect("Failed to get EthClient");
	let anvil = Anvil::new().port(eth_client.rpc_port()).spawn();
	println!("Anvil running at `{}`", anvil.endpoint());

	// set funded signer
	let signer = anvil.keys()[0].clone();
	let mut provider = scaffold.eth_client.unwrap().rpc_provider().clone();
	let mut wallet: &mut EthereumWallet = provider.wallet_mut();
	wallet.register_default_signer(LocalSigner::from(signer));

	let contract = AtomicBridgeInitiator::deploy(&provider)
		.await
		.expect("Failed to deploy contract");

	let expected_address = address!("5fbdb2315678afecb367f032d93f642f64180aa3");
	assert_eq!(contract.address(), &expected_address);

	//some data to set for the recipient.
	let recipient = address!("70997970c51812dc3a010c7d01b50e0d17dc79c8");
	let recipient_bytes: Vec<u8> = recipient.to_string().as_bytes().to_vec();

	let secret = "secret".to_string();
	let hash_lock = keccak256(secret.as_bytes());
	let hash_lock: [u8; 32] = hash_lock.into();

	let _ = eth_client
		.initiate_bridge_transfer(
			InitiatorAddress(EthAddress(expected_address)),
			RecipientAddress(recipient_bytes),
			HashLock(hash_lock),
			TimeLock(1000),
			Amount(42),
		)
		.await
		.expect("Failed to initiate bridge transfer");

	let bridge_transfer_details = eth_client
		.get_bridge_transfer_details(BridgeTransferId([0u8; 32]))
		.await
		.expect("Failed to get bridge transfer details");

	let secret = "secret".to_string();
	let hash_lock = keccak256(secret.as_bytes());
	let hash_lock: [u8; 32] = hash_lock.into();

	// let _ = eth_client
	// 	.complete_bridge_transfer()
	// 	.await
	// 	.expect
}
