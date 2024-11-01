use alloy::{
	network::EthereumWallet, providers::ProviderBuilder, signers::local::PrivateKeySigner,
};
use alloy_primitives::{Address, U256};
use bridge_config::{
	common::{eth::EthConfig, movement::MovementConfig},
	Config as BridgeConfig,
};
use bridge_service::{
	chains::ethereum::types::{
		AtomicBridgeCounterpartyMOVE, AtomicBridgeInitiatorMOVE, EthAddress,
	},
	chains::ethereum::utils::{send_transaction, send_transaction_rules},
	types::TimeLock,
};
use hex::ToHex;
use rand::Rng;
use std::{
	env, fs,
	io::Write,
	path::PathBuf,
	process::{Command, Stdio},
};

pub async fn setup(mut config: BridgeConfig) -> Result<BridgeConfig, anyhow::Error> {
	//Setup Eth config
	setup_local_ethereum(&mut config.eth).await?;
	deploy_local_movement_node(&mut config.movement)?;
	Ok(config)
}

pub async fn setup_local_ethereum(config: &mut EthConfig) -> Result<(), anyhow::Error> {
	let signer_private_key = config.signer_private_key.parse::<PrivateKeySigner>()?;
	let rpc_url = config.eth_rpc_connection_url();

	tracing::info!("Bridge deploy setup_local_ethereum");
	config.eth_initiator_contract =
		deploy_eth_initiator_contract(signer_private_key.clone(), &rpc_url)
			.await
			.to_string();
	tracing::info!("Bridge deploy after intiator");
	config.eth_counterparty_contract =
		deploy_counterpart_contract(signer_private_key.clone(), &rpc_url)
			.await
			.to_string();
	let move_token_contract = deploy_movetoken_contract(signer_private_key.clone(), &rpc_url).await;
	config.eth_move_token_contract = move_token_contract.to_string();

	initialize_initiator_contract(
		signer_private_key.clone(),
		&rpc_url,
		&config.eth_initiator_contract,
		EthAddress(move_token_contract),
		EthAddress(signer_private_key.address()),
		*TimeLock(config.time_lock_secs),
		config.gas_limit,
		config.transaction_send_retries,
	)
	.await?;
	Ok(())
}

async fn deploy_eth_initiator_contract(
	signer_private_key: PrivateKeySigner,
	rpc_url: &str,
) -> Address {
	let rpc_provider = ProviderBuilder::new()
		.with_recommended_fillers()
		.wallet(EthereumWallet::from(signer_private_key.clone()))
		.on_builtin(rpc_url)
		.await
		.expect("Error during provider creation");

	let contract = AtomicBridgeInitiatorMOVE::deploy(rpc_provider.clone())
		.await
		.expect("Failed to deploy AtomicBridgeInitiatorMOVE");
	tracing::info!("initiator_contract address: {}", contract.address().to_string());
	contract.address().to_owned()
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
	let contract = AtomicBridgeCounterpartyMOVE::deploy(rpc_provider.clone())
		.await
		.expect("Failed to deploy AtomicBridgeCounterpartyMOVE");
	tracing::info!("counterparty_contract address: {}", contract.address().to_string());
	contract.address().to_owned()
}

async fn initialize_initiator_contract(
	signer_private_key: PrivateKeySigner,
	rpc_url: &str,
	initiator_contract_address: &str,
	move_token: EthAddress,
	owner: EthAddress,
	timelock: u64,
	gas_limit: u64,
	transaction_send_retries: u32,
) -> Result<(), anyhow::Error> {
	tracing::info!("Setup Eth initialize_initiator_contract with timelock:{timelock});");
	let signer_address = signer_private_key.address();

	let rpc_provider = ProviderBuilder::new()
		.with_recommended_fillers()
		.wallet(EthereumWallet::from(signer_private_key))
		.on_builtin(rpc_url)
		.await
		.expect("Error during provider creation");
	let initiator_contract =
		AtomicBridgeInitiatorMOVE::new(initiator_contract_address.parse()?, rpc_provider);

	let call =
		initiator_contract.initialize(move_token.0, owner.0, U256::from(timelock), U256::from(100));
	send_transaction(
		call,
		signer_address,
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
