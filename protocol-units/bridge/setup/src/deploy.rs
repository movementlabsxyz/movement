use alloy::network::EthereumWallet;
use alloy::providers::ProviderBuilder;
use alloy::signers::local::PrivateKeySigner;
use alloy_primitives::Address;
use alloy_primitives::U256;
use bridge_config::common::eth::EthConfig;
use bridge_config::common::movement::MovementConfig;
use bridge_config::Config as BridgeConfig;
use bridge_service::chains::ethereum::types::AtomicBridgeCounterparty;
use bridge_service::chains::ethereum::types::AtomicBridgeInitiator;
use bridge_service::chains::ethereum::types::EthAddress;
use bridge_service::chains::ethereum::types::WETH9;
use bridge_service::chains::ethereum::utils::{send_transaction, send_transaction_rules};
use bridge_service::types::TimeLock;
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
	let eth_weth_contract = deploy_weth_contract(signer_private_key.clone(), &rpc_url).await;
	config.eth_weth_contract = eth_weth_contract.to_string();

	initialize_initiator_contract(
		signer_private_key.clone(),
		&rpc_url,
		&config.eth_initiator_contract,
		EthAddress(eth_weth_contract),
		EthAddress(signer_private_key.address()),
		*TimeLock(1),
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

	let contract = AtomicBridgeInitiator::deploy(rpc_provider.clone())
		.await
		.expect("Failed to deploy AtomicBridgeInitiator");
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

async fn initialize_initiator_contract(
	signer_private_key: PrivateKeySigner,
	rpc_url: &str,
	initiator_contract_address: &str,
	weth: EthAddress,
	owner: EthAddress,
	timelock: u64,
	gas_limit: u64,
	transaction_send_retries: u32,
) -> Result<(), anyhow::Error> {
	let rpc_provider = ProviderBuilder::new()
		.with_recommended_fillers()
		.wallet(EthereumWallet::from(signer_private_key))
		.on_builtin(rpc_url)
		.await
		.expect("Error during provider creation");
	let initiator_contract =
		AtomicBridgeInitiator::new(initiator_contract_address.parse()?, rpc_provider);

	let call = initiator_contract.initialize(weth.0, owner.0, U256::from(timelock));
	send_transaction(call, &send_transaction_rules(), transaction_send_retries, gas_limit.into())
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
			"custom",
			"--rest-url",
			"http://localhost:30731/v1",
			"--faucet-url",
			"http://localhost:30732",
			"--private-key",
			"0x5754431205b8abc443a7a877a70d6e5e67eba8e5e40b0436bff5a9b6ab4a7887",
			"--assume-yes"
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

	let random_seed = rand::thread_rng().gen_range(0, 1000000).to_string();
	println!("Publish random_seed: {}", random_seed);
	let resource_output = Command::new("movement")
		.args(&[
			"account",
			"derive-resource-account-address",
			"--address",
			address,
			"--seed",
			&random_seed,
		])
		.stdout(Stdio::piped())
		.stderr(Stdio::piped())
		.output()
		.expect("Failed to execute command");
	println!("After movement account done.");

	// Print the output of the resource address command for debugging
	if !resource_output.stdout.is_empty() {
		println!(
			"Movement account Publish stdout: {}",
			String::from_utf8_lossy(&resource_output.stdout)
		);
	}
	if !resource_output.stderr.is_empty() {
		eprintln!(
			"Movement account Publish stderr: {}",
			String::from_utf8_lossy(&resource_output.stderr)
		);
	}

	// Extract the resource address from the JSON output
	let resource_output_str = String::from_utf8_lossy(&resource_output.stdout);
	let resource_address = resource_output_str
		.lines()
		.find(|line| line.contains("\"Result\""))
		.and_then(|line| line.split('"').nth(3))
		.expect("Failed to extract the resource account address");

	// Ensure the address has a "0x" prefix

	let formatted_resource_address = if resource_address.starts_with("0x") {
		resource_address.to_string()
	} else {
		format!("0x{}", resource_address)
	};

	// Set counterparty module address to resource address, for function calls:
	println!("Publish Derived resource address: {}", formatted_resource_address);
	config.movement_native_address = formatted_resource_address.clone();

	let current_dir = env::current_dir().expect("Failed to get current directory");
	println!("Publish Current directory: {:?}", current_dir);

	//TODO Ack to make it works now but the path management should be uniform from cargo test to process compose test.
	let mut move_toml_path = PathBuf::from(current_dir);
	if config.mvt_init_network == "local" {
		move_toml_path.push("../move-modules/Move.toml")
	} else {
		move_toml_path.push("protocol-units/bridge/move-modules/Move.toml")
	};

	println!("Move move_toml_path: {move_toml_path:?}",);

	// Read the existing content of Move.toml
	let move_toml_content =
		fs::read_to_string(&move_toml_path).expect("Failed to read Move.toml file");

	// Update the content of Move.toml with the new addresses
	let updated_content = move_toml_content
		.lines()
		.map(|line| match line {
			_ if line.starts_with("resource_addr = ") => {
				println!("Update resource_addr with :{formatted_resource_address}");
				format!(r#"resource_addr = "{}""#, formatted_resource_address)
			}
			_ if line.starts_with("atomic_bridge = ") => {
				format!(r#"atomic_bridge = "{}""#, formatted_resource_address)
			}
			_ if line.starts_with("moveth = ") => {
				format!(r#"moveth = "{}""#, formatted_resource_address)
			}
			_ if line.starts_with("master_minter = ") => {
				format!(r#"master_minter = "{}""#, formatted_resource_address)
			}
			_ if line.starts_with("minter = ") => {
				format!(r#"minter = "{}""#, formatted_resource_address)
			}
			_ if line.starts_with("admin = ") => {
				format!(r#"admin = "{}""#, formatted_resource_address)
			}
			_ if line.starts_with("origin_addr = ") => {
				format!(r#"origin_addr = "{}""#, address)
			}
			_ if line.starts_with("source_account = ") => {
				format!(r#"source_account = "{}""#, address)
			}
			_ => line.to_string(),
		})
		.collect::<Vec<_>>()
		.join("\n");

	// Write the updated content back to Move.toml
	fs::write(&move_toml_path, updated_content.as_bytes())
		.expect("Failed to write updated Move.toml file");

	// let mut file =
	// 	fs::File::create(&move_toml_path).expect("Failed to open Move.toml file for writing");
	// file.write_all(updated_content.as_bytes())
	// 	.expect("Failed to write updated Move.toml file");

	println!(
		"Publis args:{:?}",
		&[
			"move",
			"create-resource-account-and-publish-package",
			"--assume-yes",
			"--address-name",
			"moveth",
			"--seed",
			&random_seed,
			"--package-dir",
			move_toml_path.parent().unwrap().to_str().unwrap(),
		]
	);

	println!("Publish Move.toml updated successfully.");

	let output2 = Command::new("movement")
		.args(&[
			"move",
			"create-resource-account-and-publish-package",
			"--assume-yes",
			"--address-name",
			"moveth",
			"--seed",
			&random_seed,
			"--package-dir",
			move_toml_path.parent().unwrap().to_str().unwrap(),
		])
		.stdout(Stdio::piped())
		.stderr(Stdio::piped())
		.output()
		.expect("Publish Failed to execute command");

	if !output2.stdout.is_empty() {
		eprintln!("Movement move Publish stdout: {}", String::from_utf8_lossy(&output2.stdout));
	}

	if !output2.stderr.is_empty() {
		eprintln!("Movement move Publish stderr: {}", String::from_utf8_lossy(&output2.stderr));
	}

	// if movement_dir.exists() {
	// 	fs::remove_dir_all(movement_dir).expect("Failed to delete .movement directory");
	// 	println!("Publish .movement directory deleted successfully.");
	// }

	// Read the existing content of Move.toml
	let move_toml_content =
		fs::read_to_string(&move_toml_path).expect("Failed to read Move.toml file");

	// Directly assign the address
	let final_address = "0xcafe";

	// Directly assign the formatted resource address
	let final_formatted_resource_address =
		"0xc3bb8488ab1a5815a9d543d7e41b0e0df46a7396f89b22821f07a4362f75ddc5";

	let updated_content = move_toml_content
		.lines()
		.map(|line| match line {
			_ if line.starts_with("resource_addr = ") => {
				format!(r#"resource_addr = "{}""#, final_formatted_resource_address)
			}
			_ if line.starts_with("atomic_bridge = ") => {
				format!(r#"atomic_bridge = "{}""#, final_formatted_resource_address)
			}
			_ if line.starts_with("moveth = ") => {
				format!(r#"moveth = "{}""#, final_formatted_resource_address)
			}
			_ if line.starts_with("master_minter = ") => {
				format!(r#"master_minter = "{}""#, final_formatted_resource_address)
			}
			_ if line.starts_with("minter = ") => {
				format!(r#"minter = "{}""#, final_formatted_resource_address)
			}
			_ if line.starts_with("admin = ") => {
				format!(r#"admin = "{}""#, final_formatted_resource_address)
			}
			_ if line.starts_with("origin_addr = ") => {
				format!(r#"origin_addr = "{}""#, final_address)
			}
			_ if line.starts_with("pauser = ") => {
				format!(r#"pauser = "{}""#, "0xdafe")
			}
			_ if line.starts_with("denylister = ") => {
				format!(r#"denylister = "{}""#, "0xcade")
			}
			_ => line.to_string(),
		})
		.collect::<Vec<_>>()
		.join("\n");

	// Write the updated content back to Move.toml
	let mut file =
		fs::File::create(&move_toml_path).expect("Failed to open Move.toml file for writing");
	file.write_all(updated_content.as_bytes())
		.expect("Failed to write updated Move.toml file");

	println!("Publish Move.toml addresses updated successfully at the end of the test.");

	Ok(())
}
