use alloy::dyn_abi::DynSolValue;
use alloy::network::EthereumWallet;
use alloy::providers::ProviderBuilder;
use alloy::signers::local::PrivateKeySigner;
use alloy_primitives::Address;
use alloy_primitives::FixedBytes;
use alloy_primitives::U256;
use bridge_config::common::eth::EthConfig;
use bridge_config::common::movement::MovementConfig;
use bridge_config::Config as BridgeConfig;
use bridge_service::chains::ethereum::types::AtomicBridgeCounterparty;
use bridge_service::chains::ethereum::types::AtomicBridgeInitiator;
use bridge_service::chains::ethereum::types::AtomicBridgeInitiator::poolBalanceReturn;
use bridge_service::chains::ethereum::types::CounterpartyContract;
use bridge_service::chains::ethereum::types::EthAddress;
use bridge_service::chains::ethereum::types::WETH9;
use bridge_service::chains::ethereum::utils::{send_transaction, send_transaction_rules};
use bridge_service::types::TimeLock;
use ethabi::{Contract, Token};
use hex::ToHex;
use rand::Rng;
use serde_json::{from_str, Value};
use std::{
	env, fs,
	io::Write,
	path::PathBuf,
	process::{Command, Stdio},
};

// Proxy contract to be able to call bridge contract.
alloy::sol!(
	#[allow(missing_docs)]
	#[sol(rpc)]
	ProxyAdmin,
	"../service/abis/ProxyAdmin.json"
);

alloy::sol!(
	#[allow(missing_docs)]
	#[sol(rpc)]
	TransparentUpgradeableProxy,
	"../service/abis/TransparentUpgradeableProxy.json"
);

pub async fn setup(mut config: BridgeConfig) -> Result<BridgeConfig, anyhow::Error> {
	//Setup Eth config
	setup_local_ethereum(&mut config).await?;
	deploy_local_movement_node(&mut config.movement)?;
	Ok(config)
}

pub async fn setup_local_ethereum(config: &mut BridgeConfig) -> Result<(), anyhow::Error> {
	let signer_private_key = config.eth.signer_private_key.parse::<PrivateKeySigner>()?;
	let rpc_url = config.eth.eth_rpc_connection_url();

	tracing::info!("Bridge deploy setup_local_ethereum");
	config.eth.eth_initiator_contract = deploy_eth_initiator_contract(config).await?.to_string();
	tracing::info!("Bridge deploy after intiator");
	tracing::info!("Signer private key: {:?}", signer_private_key.address());
	config.eth.eth_counterparty_contract =
		deploy_counterpart_contract(signer_private_key.clone(), &rpc_url)
			.await
			.to_string();
	let eth_weth_contract = deploy_weth_contract(signer_private_key.clone(), &rpc_url).await;
	config.eth.eth_weth_contract = eth_weth_contract.to_string();

	initialize_eth_contracts(
		signer_private_key.clone(),
		&rpc_url,
		&config.eth.eth_initiator_contract,
		&config.eth.eth_counterparty_contract,
		EthAddress(eth_weth_contract),
		EthAddress(signer_private_key.address()),
		*TimeLock(config.eth.time_lock_secs),
		config.eth.gas_limit,
		config.eth.transaction_send_retries,
	)
	.await?;
	Ok(())
}

async fn deploy_eth_initiator_contract(
	config: &mut BridgeConfig,
) -> Result<Address, anyhow::Error> {
	let signer_private_key = config.eth.signer_private_key.parse::<PrivateKeySigner>()?;
	let rpc_url = config.eth.eth_rpc_connection_url();

	let rpc_provider = ProviderBuilder::new()
		.with_recommended_fillers()
		.wallet(EthereumWallet::from(signer_private_key.clone()))
		.on_builtin(&rpc_url)
		.await
		.expect("Error during provider creation");

	// Deploy the ProxyAdmin contract
	//	let proxy_admin_signer = config.
	// let proxy_admin =
	// 	ProxyAdmin::deploy_builder(rpc_provider.clone(), signer_private_key.address());
	// let proxy_admin_address = proxy_admin.deploy().await.expect("Failed to deploy ProxyAdmin");

	let weth = WETH9::deploy(rpc_provider.clone()).await.expect("Failed to deploy WETH9");
	tracing::info!("weth_contract address: {}", weth.address().to_string());

	let initiator_contract = AtomicBridgeInitiator::deploy(rpc_provider.clone()).await?;
	tracing::info!("initiator_contract address: {}", initiator_contract.address().to_string());

	// Initiator initialize the contract data
	// let initializer_data = DynSolValue::Tuple(vec![
	// 	DynSolValue::Address(*weth.address()),
	// 	DynSolValue::Address(signer_private_key.address()),
	// 	DynSolValue::Uint(U256::from(config.time_lock_secs), 256),
	// 	DynSolValue::Uint(U256::from(100 as u128 * 100_000_000 as u128), 256),
	// ]);

	// Load the ABI from a JSON file or inline JSON
	//	let contract_abi = include_bytes!("../../service/abis/AtomicBridgeInitiator.json");
	let path = "/home/pdelrieu/dev/blockchain/movement/github/PR/state_logic/movement/protocol-units/bridge/service/abis/AtomicBridgeInitiator.json";
	let data = fs::read_to_string(path).expect("Unable to read ABI file");

	// Parse the JSON data
	let v: Value = from_str(&data).expect("Unable to parse JSON");

	// Extract the "abi" field
	let abi = v["abi"].to_string();

	let contract = Contract::load(abi.as_bytes()).expect("Incorrect ABI");
	let function = contract.function("initialize").expect("Function must exist in ABI");
	let tokens = vec![
		Token::Address(ethabi::Address::from_slice(weth.address().as_slice())),
		Token::Address(ethabi::Address::from_slice(signer_private_key.address().as_slice())),
		Token::Uint(ethabi::Uint::from(config.eth.time_lock_secs)),
		Token::Uint(ethabi::Uint::from(100 as u128 * 100_000_000 as u128)),
	];

	// Encode the function call
	let initializer_data = function.encode_input(&tokens).unwrap();

	// Deploy TransparentUpgradeableProxy for AtomicBridgeCounterparty
	let proxy_admin_signer = config.testing.eth_well_known_account_private_keys[4]
		.clone()
		.parse::<PrivateKeySigner>()
		.unwrap();
	let upgradeable_proxy_counterparty = TransparentUpgradeableProxy::deploy(
		rpc_provider.clone(),          // The provider (same one used for deployment)
		*initiator_contract.address(), // Address of the contract
		proxy_admin_signer.address(),
		initializer_data.into(),
	)
	.await?;

	// let call = upgradeable_proxy_counterparty
	// 	.upgradeToAndCall(*initiator_contract.address(), initializer_data)
	// 	.await
	// 	.expect("Failed to initialize TransparentUpgradeableProxy for AtomicBridgeCounterparty");
	// send_transaction(call, &send_transaction_rules(), 10, config.gas_limit.into())
	// 	.await
	// 	.expect("Failed to send transaction");

	//test proxy call
	let initiator_contract =
		AtomicBridgeInitiator::new(*upgradeable_proxy_counterparty.address(), rpc_provider.clone());

	let builder = initiator_contract.poolBalance();
	let pool_balance = builder.call().await?._0.to_string();
	println!("ICI poolBalance:{pool_balance}");
	let builder = initiator_contract.initiatorTimeLockDuration();
	let initiator_time_lock_duration = builder.call().await?._0.to_string();
	println!("ICI poolBalance:{initiator_time_lock_duration}");

	let call = initiator_contract
		.initiateBridgeTransfer(U256::from(0), FixedBytes([3; 32]), FixedBytes([2; 32]))
		.value(U256::from(1))
		.from(signer_private_key.address());
	send_transaction(call, &send_transaction_rules(), 10, config.eth.gas_limit.into())
		.await
		.expect("Failed to send transaction");

	let builder = initiator_contract.owner();
	let owner = builder.call().await?._0.to_string();
	println!("ICI owner:{owner}");
	println!("ICI signer_private_key.address():{}", signer_private_key.address());
	println!("ICI upgradeable_proxy_counterparty:{}", upgradeable_proxy_counterparty.address());

	let call = initiator_contract.setCounterpartyAddress(*upgradeable_proxy_counterparty.address());
	send_transaction(call, &send_transaction_rules(), 10, config.eth.gas_limit.into())
		.await
		.expect("Failed to send transaction");

	println!("ICICIC call initiator done");

	Ok(upgradeable_proxy_counterparty.address().to_owned())
}

async fn deploy_counterpart_contract(
	signer_private_key: PrivateKeySigner,
	rpc_url: &str,
) -> Address {
	let rpc_provider = ProviderBuilder::new()
		.with_recommended_fillers()
		.wallet(EthereumWallet::from(signer_private_key))
		.on_builtin(rpc_url)
		.await
		.expect("Error during provider creation");
	let contract = AtomicBridgeCounterparty::deploy(rpc_provider.clone())
		.await
		.expect("Failed to deploy AtomicBridgeInitiator");
	tracing::info!("counterparty_contract address: {}", contract.address().to_string());
	contract.address().to_owned()
}

async fn deploy_weth_contract(signer_private_key: PrivateKeySigner, rpc_url: &str) -> Address {
	let rpc_provider = ProviderBuilder::new()
		.with_recommended_fillers()
		.wallet(EthereumWallet::from(signer_private_key.clone()))
		.on_builtin(rpc_url)
		.await
		.expect("Error during provider creation");
	let weth = WETH9::deploy(rpc_provider).await.expect("Failed to deploy WETH9");
	tracing::info!("weth_contract address: {}", weth.address().to_string());
	weth.address().to_owned()
}

async fn initialize_eth_contracts(
	signer_private_key: PrivateKeySigner,
	rpc_url: &str,
	initiator_contract_address: &str,
	counterpart_contract_address: &str,
	weth: EthAddress,
	owner: EthAddress,
	timelock: u64,
	gas_limit: u64,
	transaction_send_retries: u32,
) -> Result<(), anyhow::Error> {
	tracing::info!("Setup Eth initialize_initiator_contract with timelock:{timelock});");

	let rpc_provider = ProviderBuilder::new()
		.with_recommended_fillers()
		.wallet(EthereumWallet::from(signer_private_key.clone()))
		.on_builtin(rpc_url)
		.await
		.expect("Error during provider creation");
	let initiator_contract =
		AtomicBridgeInitiator::new(initiator_contract_address.parse()?, rpc_provider.clone());

	let call = initiator_contract.initialize(
		weth.0,
		owner.0,
		U256::from(timelock),
		U256::from(100 as u128 * 100_000_000 as u128), // Set the eth pool to 100 eth.
	);
	send_transaction(call, &send_transaction_rules(), transaction_send_retries, gas_limit.into())
		.await
		.expect("Failed to send transaction");

	//update the Initiator contract with the Counterpart address
	let call = initiator_contract.setCounterpartyAddress(counterpart_contract_address.parse()?);
	send_transaction(call, &send_transaction_rules(), transaction_send_retries, gas_limit.into())
		.await
		.expect("Failed to send transaction");

	let pool_balance: poolBalanceReturn = initiator_contract.poolBalance().call().await?;
	tracing::info!("Pool balance: {:?}", pool_balance._0);

	let counterpart_contract =
		CounterpartyContract::new(counterpart_contract_address.parse()?, rpc_provider);
	let call = counterpart_contract.initialize(
		initiator_contract_address.parse()?,
		signer_private_key.address(),
		U256::from(timelock),
	);
	let _ = send_transaction(
		call,
		&send_transaction_rules(),
		transaction_send_retries,
		gas_limit.into(),
	)
	.await
	.expect("Failed to send transaction");

	Ok(())
}

pub fn deploy_local_movement_node(config: &mut MovementConfig) -> Result<(), anyhow::Error> {
	println!("Start deploy_local_movement_node");
	let mut process = Command::new("movement") //--network
		.args(&[
			"init",
			"--network",
			&config.mvt_init_network,
			"--rest-url",
			&config.mvt_rpc_connection_url(),
			"--faucet-url",
			&config.mvt_faucet_connection_url(),
			"--assume-yes",
		])
		.stdin(Stdio::piped())
		.stdout(Stdio::piped())
		.stderr(Stdio::piped())
		.spawn()
		.expect("Failed to execute command");

	let stdin: &mut std::process::ChildStdin =
		process.stdin.as_mut().expect("Failed to open stdin");

	//	stdin.write_all(b"local\n").expect("Failed to write to stdin");

	let private_key_bytes = config.movement_signer_key.to_bytes();
	let private_key_hex = format!("0x{}", private_key_bytes.encode_hex::<String>());
	let _ = stdin.write_all(format!("{}\n", private_key_hex).as_bytes());

	let addr_output = process.wait_with_output().expect("Failed to read command output");
	if !addr_output.stdout.is_empty() {
		println!("Move init Publish stdout: {}", String::from_utf8_lossy(&addr_output.stdout));
	}

	if !addr_output.stderr.is_empty() {
		eprintln!("Move init Publish stderr: {}", String::from_utf8_lossy(&addr_output.stderr));
	}

	let addr_output_str = String::from_utf8_lossy(&addr_output.stderr);
	let address = addr_output_str
		.split_whitespace()
		.find(|word| word.starts_with("0x"))
		.expect("Failed to extract the Movement account address");

	println!("Publish Extracted address: {}", address);

	Ok(())
}
