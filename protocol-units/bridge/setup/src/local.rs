use alloy::node_bindings::{Anvil, AnvilInstance};
use alloy::signers::local::PrivateKeySigner;
use aptos_sdk::types::LocalAccount;
use bridge_config::common::eth::EthConfig;
use bridge_config::common::movement::MovementConfig;
use bridge_config::common::testing::TestingConfig;
use bridge_config::Config as BridgeConfig;
use hex::ToHex;
use rand::prelude::*;
use std::{
	env, fs,
	io::Write,
	path::PathBuf,
	process::{Command, Stdio},
};
use tokio::io::AsyncBufReadExt;
use tokio::io::BufReader;
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
) -> Result<tokio::process::Child, anyhow::Error> {
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

	let delete_dir_cmd = TokioCommand::new("sh")
		.arg("-c")
		.arg("if [ -d '.movement/config.yaml' ]; then rm -rf .movement/config.yaml; fi")
		.output()
		.await?;

	if !delete_dir_cmd.status.success() {
		println!("Failed to delete .movement directory: {:?}", delete_dir_cmd.stderr);
	} else {
		println!(".movement directory deleted if it was present.");
	}

	let (setup_complete_tx, setup_complete_rx) = tokio::sync::oneshot::channel();
	let mut child = TokioCommand::new("movement")
		.args(&["node", "run-local-testnet", "--force-restart", "--assume-yes"])
		.stdout(Stdio::piped())
		.stderr(Stdio::piped())
		.spawn()?;

	let stdout = child.stdout.take().expect("Failed to capture stdout");
	let stderr = child.stderr.take().expect("Failed to capture stderr");

	tokio::task::spawn(async move {
		let mut stdout_reader = BufReader::new(stdout).lines();
		let mut stderr_reader = BufReader::new(stderr).lines();

		loop {
			tokio::select! {
				line = stdout_reader.next_line() => {
					match line {
						Ok(Some(line)) => {
							println!("STDOUT: {}", line);
							if line.contains("Setup is complete") {
								println!("Testnet is up and running!");
								let _ = setup_complete_tx.send(());
																return Ok(());
							}
						},
						Ok(_) => {
							return Err(anyhow::anyhow!("Unexpected end of stdout stream"));
						},
						Err(e) => {
							return Err(anyhow::anyhow!("Error reading stdout: {}", e));
						}
					}
				},
				line = stderr_reader.next_line() => {
					match line {
						Ok(Some(line)) => {
							println!("STDERR: {}", line);
							if line.contains("Setup is complete") {
								println!("Testnet is up and running!");
								let _ = setup_complete_tx.send(());
																return Ok(());
							}
						},
						Ok(_) => {
							return Err(anyhow::anyhow!("Unexpected end of stderr stream"));
						}
						Err(e) => {
							return Err(anyhow::anyhow!("Error reading stderr: {}", e));
						}
					}
				}
			}
		}
	});

	setup_complete_rx.await.expect("Failed to receive setup completion signal");
	println!("Movement node startup complete message received.");

	std::thread::sleep(std::time::Duration::from_secs(7));

	let mut rng = ::rand::rngs::StdRng::from_seed([3u8; 32]);
	let signer = LocalAccount::generate(&mut rng);
	config.movement_signer_address = signer.private_key().clone();

	Ok(child)
}

pub fn init_local_movement_node(config: &mut MovementConfig) -> Result<(), anyhow::Error> {
	let mut process = Command::new("movement") //--network
		.args(&[
			"init",
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

	stdin.write_all(b"local\n").expect("Failed to write to stdin");

	let private_key_bytes = config.movement_signer_address.to_bytes();
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
