use alloy::node_bindings::{Anvil, AnvilInstance};
use alloy::signers::local::PrivateKeySigner;
use anyhow::Context;
use aptos_sdk::types::account_address::AccountAddress;
use aptos_sdk::types::LocalAccount;
use bridge_config::common::eth::EthConfig;
use bridge_config::common::movement::MovementConfig;
use bridge_config::common::testing::TestingConfig;
use bridge_config::Config as BridgeConfig;
use commander::run_command;
use rand::prelude::*;
use std::{
	env, fs,
	io::Write,
	path::PathBuf,
	process::{Command, Stdio},
};
use tokio::process::Command as TokioCommand;

pub async fn setup(
	mut config: BridgeConfig,
) -> Result<(BridgeConfig, AnvilInstance), anyhow::Error> {
	tracing::info!("Bridge local setup");
	//Eth init: Start anvil.
	let anvil = setup_eth(&mut config.eth, &mut config.testing);
	tracing::info!("Bridge after anvil");

	//By default the setup deosn't start the Movement node.
	Ok((config, anvil))
}

pub fn setup_eth(config: &mut EthConfig, testing_config: &mut TestingConfig) -> AnvilInstance {
	let anvil = Anvil::new().port(config.eth_rpc_connection_port).spawn();
	//update config with Anvil address
	let signer: PrivateKeySigner = anvil.keys()[1].clone().into();
	config.signer_private_key = signer.to_bytes().to_string();
	for key in anvil.keys().iter().skip(2) {
		let privkey: PrivateKeySigner = (key.clone()).into();
		testing_config
			.eth_well_known_account_private_keys
			.push(privkey.to_bytes().to_string());
	}

	anvil
}

pub async fn setup_movement_node(
	config: &mut MovementConfig,
) -> Result<tokio::task::JoinHandle<Result<String, anyhow::Error>>, anyhow::Error> {
	//kill existing process if any.
	let kill_cmd = TokioCommand::new("sh")
			.arg("-c")
			.arg("PID=$(ps aux | grep 'movement node run-local-testnet' | grep -v grep | awk '{print $2}' | head -n 1); if [ -n \"$PID\" ]; then kill -9 $PID; fi")
			.output()
			.await?;

	if !kill_cmd.status.success() {
		tracing::info!("Failed to kill running movement process: {:?}", kill_cmd.stderr);
	} else {
		tracing::info!("Movement process killed if it was running.");
	}

	// let delete_dir_cmd = TokioCommand::new("sh")
	// 	.arg("-c")
	// 	.arg("if [ -d '.movement' ]; then rm -rf .movement; fi")
	// 	.output()
	// 	.await?;

	// if !delete_dir_cmd.status.success() {
	// 	println!("Failed to delete .movement directory: {:?}", delete_dir_cmd.stderr);
	// } else {
	// 	println!(".movement directory deleted if it was present.");
	// }

	let movement_join_handle = tokio::task::spawn(async move {
		run_command(
			"movement",
			&vec!["node", "run-local-testnet", "--force-restart", "--assume-yes"],
		)
		.await
		.context("Failed to start Movement node from CLI")
	});

	let mut rng = ::rand::rngs::StdRng::from_seed([3u8; 32]);
	let signer = LocalAccount::generate(&mut rng);
	let private_key_hex = hex::encode(signer.private_key().to_bytes());
	config.movement_signer_address = private_key_hex;

	Ok(movement_join_handle)
}

pub fn init_local_movement_node(config: &mut MovementConfig) -> Result<(), anyhow::Error> {
	let mut process = Command::new("movement")
		.args(&["init"])
		.stdin(Stdio::piped())
		.stdout(Stdio::piped())
		.stderr(Stdio::piped())
		.spawn()
		.expect("Failed to execute command");

	let stdin: &mut std::process::ChildStdin =
		process.stdin.as_mut().expect("Failed to open stdin");

	let movement_dir = PathBuf::from(".movement");

	if movement_dir.exists() {
		stdin.write_all(b"yes\n").expect("Failed to write to stdin");
	}

	stdin.write_all(b"local\n").expect("Failed to write to stdin");

	let private_key_hex = config.movement_signer_address.clone();
	let _ = stdin.write_all(format!("{}\n", private_key_hex).as_bytes());

	let addr_output = process.wait_with_output().expect("Failed to read command output");

	if !addr_output.stdout.is_empty() {
		println!("Publish stdout: {}", String::from_utf8_lossy(&addr_output.stdout));
	}

	if !addr_output.stderr.is_empty() {
		eprintln!("Publish stderr: {}", String::from_utf8_lossy(&addr_output.stderr));
	}
	let addr_output_str = String::from_utf8_lossy(&addr_output.stderr);
	let address = addr_output_str
		.split_whitespace()
		.find(|word| word.starts_with("0x"))
		.expect("Failed to extract the Movement account address");

	println!("Publish Extracted address: {}", address);

	let random_seed = rand::thread_rng().gen_range(0, 1000000).to_string();
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

	// Print the output of the resource address command for debugging
	if !resource_output.stdout.is_empty() {
		println!("Publish stdout: {}", String::from_utf8_lossy(&resource_output.stdout));
	}
	if !resource_output.stderr.is_empty() {
		eprintln!("Publish stderr: {}", String::from_utf8_lossy(&resource_output.stderr));
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
	let native_address = AccountAddress::from_hex_literal(&formatted_resource_address)?;
	println!("Publish Derived resource address: {}", formatted_resource_address);
	config.movement_native_address = formatted_resource_address.clone();

	let current_dir = env::current_dir().expect("Failed to get current directory");
	println!("Publish Current directory: {:?}", current_dir);

	let move_toml_path = PathBuf::from("../move-modules/Move.toml");

	// Read the existing content of Move.toml
	let move_toml_content =
		fs::read_to_string(&move_toml_path).expect("Failed to read Move.toml file");

	// Update the content of Move.toml with the new addresses
	let updated_content = move_toml_content
		.lines()
		.map(|line| match line {
			_ if line.starts_with("resource_addr = ") => {
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
	let mut file =
		fs::File::create(&move_toml_path).expect("Failed to open Move.toml file for writing");
	file.write_all(updated_content.as_bytes())
		.expect("Failed to write updated Move.toml file");

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
			"../move-modules",
		])
		.stdout(Stdio::piped())
		.stderr(Stdio::piped())
		.output()
		.expect("Publish Failed to execute command");

	if !output2.stdout.is_empty() {
		eprintln!("Publish stdout: {}", String::from_utf8_lossy(&output2.stdout));
	}

	if !output2.stderr.is_empty() {
		eprintln!("Publish stderr: {}", String::from_utf8_lossy(&output2.stderr));
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
