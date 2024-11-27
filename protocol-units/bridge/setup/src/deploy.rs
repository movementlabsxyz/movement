use alloy::{
	network::EthereumWallet, providers::ProviderBuilder, signers::local::PrivateKeySigner,
};
use alloy_primitives::Address;
use alloy_primitives::U256;
use bridge_config::{common::movement::MovementConfig, Config as BridgeConfig};
use bridge_service::chains::ethereum::{
	types::{EthAddress, MockMOVEToken, NativeBridgeContract},
	utils::{send_transaction, send_transaction_rules},
};
use hex::ToHex;
use std::io::BufRead;
use std::{
	io::Write,
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
	init_movement_node(&mut config.movement)?;
	deploy_local_movement_node(&mut config.movement)?;
	Ok(config)
}

pub async fn setup_local_ethereum(config: &mut BridgeConfig) -> Result<(), anyhow::Error> {
	println!("ICI setup_local_ethereum {:?}", config.eth.eth_rpc_connection_url());
	let signer_private_key = config.eth.signer_private_key.parse::<PrivateKeySigner>()?;
	let rpc_url = config.eth.eth_rpc_connection_url();

	tracing::info!("Bridge deploy setup_local_ethereum");
	config.eth.eth_native_contract = deploy_eth_native_contract(config).await?.to_string();
	tracing::info!("Bridge deploy after intiator");
	tracing::info!("Signer private key: {:?}", signer_private_key.address());

	let move_token_contract =
		deploy_move_token_contract(signer_private_key.clone(), &rpc_url).await;
	config.eth.eth_move_token_contract = move_token_contract.to_string();

	config.eth.eth_native_contract = initialize_eth_contracts(
		signer_private_key.clone(),
		&rpc_url,
		&config.eth.eth_native_contract,
		EthAddress(move_token_contract),
		EthAddress(signer_private_key.address()),
		config.eth.gas_limit,
		config.eth.transaction_send_retries,
	)
	.await?
	.to_string();
	Ok(())
}

async fn deploy_eth_native_contract(config: &mut BridgeConfig) -> Result<Address, anyhow::Error> {
	let signer_private_key = config.eth.signer_private_key.parse::<PrivateKeySigner>()?;
	println!("ICI {:?}", config.eth.eth_rpc_connection_url());
	let rpc_url = config.eth.eth_rpc_connection_url();

	let rpc_provider = ProviderBuilder::new()
		.with_recommended_fillers()
		.wallet(EthereumWallet::from(signer_private_key.clone()))
		.on_builtin(&rpc_url)
		.await
		.expect("Error during provider creation");

	let contract = NativeBridgeContract::deploy(rpc_provider.clone())
		.await
		.expect("Failed to deploy AtomicBridgeInitiatorMOVE");
	tracing::info!("initiator_contract address: {}", contract.address().to_string());
	Ok(contract.address().to_owned())
}

async fn deploy_move_token_contract(
	signer_private_key: PrivateKeySigner,
	rpc_url: &str,
) -> Address {
	let rpc_provider = ProviderBuilder::new()
		.with_recommended_fillers()
		.wallet(EthereumWallet::from(signer_private_key.clone()))
		.on_builtin(rpc_url)
		.await
		.expect("Error during provider creation");
	let move_token = MockMOVEToken::deploy(rpc_provider)
		.await
		.expect("Failed to deploy Mock MOVE token");
	tracing::info!("Move token address: {}", move_token.address().to_string());
	move_token.address().to_owned()
}

use ethabi::{Contract, Token};
use serde_json::{from_str, Value};

async fn initialize_eth_contracts(
	signer_private_key: PrivateKeySigner,
	rpc_url: &str,
	native_bridge_contract_address: &str,
	move_token: EthAddress,
	owner: EthAddress,
	gas_limit: u64,
	transaction_send_retries: u32,
) -> Result<Address, anyhow::Error> {
	tracing::info!("Setup Eth initialize_initiator_contract.");
	let signer_address = signer_private_key.address();

	let rpc_provider = ProviderBuilder::new()
		.with_recommended_fillers()
		.wallet(EthereumWallet::from(signer_private_key.clone()))
		.on_builtin(rpc_url)
		.await
		.expect("Error during provider creation");
	let native_bridge_contract_address: Address = native_bridge_contract_address.parse()?;

	// Initialize the MockMOVEToken contract with the initiator address as the initial fund recipient
	let mock_move_token = MockMOVEToken::new(*move_token, &rpc_provider);
	let initialize_token_call = mock_move_token.initialize(owner.0).from(owner.0);

	let _ = send_transaction(
		initialize_token_call,
		signer_address,
		&send_transaction_rules(),
		transaction_send_retries,
		gas_limit.into(),
	)
	.await;

	// create proxy contracts.
	let proxy_admin = ProxyAdmin::deploy(rpc_provider.clone(), signer_address)
		.await
		.expect("Failed to deploy ProxyAdmin");
	// Deploy TransparentUpgradeableProxy for AtomicBridgeCounterparty

	// Load the ABI from a JSON file or inline JSON
	//	let contract_abi = include_bytes!("../../service/abis/AtomicBridgeInitiator.json");
	let path = "/home/pdelrieu/dev/blockchain/movement/github/PR/state_logic/both_framework/movement/protocol-units/bridge/service/abis/NativeBridge.json";
	let data = std::fs::read_to_string(path).expect("Unable to read ABI file");

	// Parse the JSON data
	let v: Value = from_str(&data).expect("Unable to parse JSON");

	// Extract the "abi" field
	let abi = v["abi"].to_string();

	let contract = Contract::load(abi.as_bytes()).expect("Incorrect ABI");
	let function = contract.function("initialize").expect("Function must exist in ABI");
	let tokens = vec![
		Token::Address(ethabi::Address::from_slice(move_token.as_slice())),
		Token::Address(ethabi::Address::from_slice(signer_address.as_slice())),
		Token::Address(ethabi::Address::from_slice(signer_address.as_slice())),
		Token::Address(ethabi::Address::from_slice(Address::ZERO.as_slice())),
	];
	// Encode the function call
	let initialize_data = function.encode_input(&tokens).unwrap();

	let upgradeable_proxy = TransparentUpgradeableProxy::deploy(
		rpc_provider.clone(),           // The provider (same one used for deployment)
		native_bridge_contract_address, // Address of the contract
		*proxy_admin.address(),
		initialize_data.clone().into(),
	)
	.await?;

	// Transfer some coin to the native contract (using the proxy address) so that it can complete transfer.
	let transfer_token_call = mock_move_token
		.transfer(*upgradeable_proxy.address(), U256::from(10000000000u64) * U256::from(100000u64))
		.from(signer_address);

	let _ = send_transaction(
		transfer_token_call,
		signer_address,
		&send_transaction_rules(),
		transaction_send_retries,
		gas_limit.into(),
	)
	.await;

	Ok(*upgradeable_proxy.address())
}

pub fn deploy_local_movement_node(config: &mut MovementConfig) -> Result<(), anyhow::Error> {
	//init_movement_node(config)?;
	update_mvt_account_address()?;
	deploy_on_movement_framework(config)?;
	Ok(())
}

pub fn init_movement_node(config: &mut MovementConfig) -> Result<(), anyhow::Error> {
	tracing::info!("Start deploy_local_movement_node rpc url:{}", config.mvt_rpc_connection_url());
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
		tracing::info!(
			"Move init Publish stdout: {}",
			String::from_utf8_lossy(&addr_output.stdout)
		);
	}

	if !addr_output.stderr.is_empty() {
		tracing::info!(
			"Move init Publish stderr: {}",
			String::from_utf8_lossy(&addr_output.stderr)
		);
	}

	let addr_output_str = String::from_utf8_lossy(&addr_output.stderr);
	let address = addr_output_str
		.split_whitespace()
		.find(|word| word.starts_with("0x"))
		.expect("Failed to extract the Movement account address");

	tracing::info!("Publish Extracted address: {}", address);

	Ok(())
}

pub fn deploy_on_movement_framework(config: &mut MovementConfig) -> Result<(), anyhow::Error> {
	tracing::info!("Before compile move modules");
	let compile_output = Command::new("movement")
		.args(&["move", "compile", "--package-dir", "protocol-units/bridge/move-modules/"])
		.stdout(Stdio::piped())
		.stderr(Stdio::piped())
		.output()?;

	if !compile_output.stdout.is_empty() {
		tracing::info!("move compile stdout: {}", String::from_utf8_lossy(&compile_output.stdout));
	}
	if !compile_output.stderr.is_empty() {
		tracing::info!("move compile stderr: {}", String::from_utf8_lossy(&compile_output.stderr));
	}
	let enable_bridge_feature_output = Command::new("movement")
			.args(&[
				"move",
				"run-script",
				"--compiled-script-path",
				"protocol-units/bridge/move-modules/build/bridge-modules/bytecode_scripts/enable_bridge_feature.mv",
				"--profile",
				"default",
				"--assume-yes",
			])
			.stdout(Stdio::piped())
			.stderr(Stdio::piped())
			.output()?;

	if !enable_bridge_feature_output.stdout.is_empty() {
		println!(
			"run-script enable_bridge_feature stdout: {}",
			String::from_utf8_lossy(&enable_bridge_feature_output.stdout)
		);
	}
	if !enable_bridge_feature_output.stderr.is_empty() {
		eprintln!(
			"run-script enable_bridge_feature stderr: {}",
			String::from_utf8_lossy(&enable_bridge_feature_output.stderr)
		);
	}

	let store_mint_burn_caps_output = Command::new("movement")
			.args(&[
				"move",
				"run-script",
				"--compiled-script-path",
				"protocol-units/bridge/move-modules/build/bridge-modules/bytecode_scripts/store_mint_burn_caps.mv",
				"--profile",
				"default",
				"--assume-yes",
			])
			.stdout(Stdio::piped())
			.stderr(Stdio::piped())
			.output()?;

	if !store_mint_burn_caps_output.stdout.is_empty() {
		println!(
			"run-script store_mint_burn_caps stdout: {}",
			String::from_utf8_lossy(&store_mint_burn_caps_output.stdout)
		);
	}
	if !store_mint_burn_caps_output.stderr.is_empty() {
		eprintln!(
			"run-script store_mint_burn_caps stderr: {}",
			String::from_utf8_lossy(&store_mint_burn_caps_output.stderr)
		);
	}

	let update_bridge_operator_output = Command::new("movement")
			.args(&[
				"move",
				"run-script",
				"--compiled-script-path",
				"protocol-units/bridge/move-modules/build/bridge-modules/bytecode_scripts/update_bridge_operator.mv",
				"--args",
				"address:0xf90391c81027f03cdea491ed8b36ffaced26b6df208a9b569e5baf2590eb9b16",
				"--profile",
				"default",
				"--assume-yes",
			])
			.stdout(Stdio::piped())
			.stderr(Stdio::piped())
			.output()?;

	if !update_bridge_operator_output.stdout.is_empty() {
		println!(
			"run-script update_bridge_operatorstdout: {}",
			String::from_utf8_lossy(&update_bridge_operator_output.stdout)
		);
	}
	if !update_bridge_operator_output.stderr.is_empty() {
		eprintln!(
			"run-script update_bridge_operator supdate_bridge_operator tderr: {}",
			String::from_utf8_lossy(&update_bridge_operator_output.stderr)
		);
	}

	let set_initiator_time_lock_script_output = Command::new("movement")
		.args(&[
			"move",
			"run-script",
			"--compiled-script-path",
			"protocol-units/bridge/move-modules/build/bridge-modules/bytecode_scripts/set_initiator_time_lock_duration.mv",
			"--args",
			"u64: 11",
			"--profile",
			"default",
			"--assume-yes",
		])
		.stdout(Stdio::piped())
		.stderr(Stdio::piped())
		.output()?;

	if !set_initiator_time_lock_script_output.stdout.is_empty() {
		println!(
			"run-script set_initiator_time_lock_duration stdout: {}",
			String::from_utf8_lossy(&update_bridge_operator_output.stdout)
		);
	}
	if !set_initiator_time_lock_script_output.stderr.is_empty() {
		eprintln!(
			"run-script set_initiator_time_lock_duration stderr: {}",
			String::from_utf8_lossy(&update_bridge_operator_output.stderr)
		);
	}

	let set_counterparty_time_lock_script_output = Command::new("movement")
		.args(&[
			"move",
			"run-script",
			"--compiled-script-path",
			"protocol-units/bridge/move-modules/build/bridge-modules/bytecode_scripts/set_counterparty_time_lock_duration.mv",
			"--args",
			"u64: 5",
			"--profile",
			"default",
			"--assume-yes",
		])
		.stdout(Stdio::piped())
		.stderr(Stdio::piped())
		.output()?;

	if !set_counterparty_time_lock_script_output.stdout.is_empty() {
		println!(
			"run-script set_counterparty_time_lock_duration stdout: {}",
			String::from_utf8_lossy(&update_bridge_operator_output.stdout)
		);
	}
	if !set_counterparty_time_lock_script_output.stderr.is_empty() {
		eprintln!(
			"run-script set_counterparty_time_lock_duration stderr: {}",
			String::from_utf8_lossy(&update_bridge_operator_output.stderr)
		);
	}

	println!("Mvt framework deployed.");

	Ok(())
}

fn update_mvt_account_address() -> Result<(), anyhow::Error> {
	let config_file_path = std::env::current_dir()?.join(".movement/config.yaml");
	let new_address = "0xA550C18";
	let mut tmp_lines: Vec<String> = Vec::new();

	// Read the contents of the file
	{
		let file = std::fs::File::open(&config_file_path)?;
		let reader = std::io::BufReader::new(file);

		let mut lines_iterator = reader.lines();
		while let Some(line) = lines_iterator.next() {
			let line = line?;
			if line.contains("account:") {
				// Replace the line with the new address value
				tmp_lines.push(format!("    account: {}", new_address));
			} else {
				// Keep the original line
				tmp_lines.push(line);
			}
		}
	}

	let mut output_file = std::fs::File::create(&config_file_path)?;
	for line in tmp_lines {
		output_file.write_all(line.as_bytes())?;
		output_file.write_all(b"\n")?; // Add newline character
	}

	Ok(())
}
