use anyhow::Result;
use aptos_sdk::types::account_address::AccountAddress;
use aptos_sdk::{
	rest_client::{Client, FaucetClient},
	types::LocalAccount,
};
use bridge_service::chains::bridge_contracts::BridgeContractError;
use bridge_service::chains::bridge_contracts::BridgeContractResult;
use bridge_service::chains::movement::utils::MovementAddress;
use bridge_service::types::Amount;
use bridge_service::types::AssetType;
use bridge_service::types::BridgeAddress;
use bridge_service::types::HashLock;
use tracing::debug;
//use bridge_service::chains::movement::client::MovementClient;
//AlloyProvider, AtomicBridgeInitiator,
use rand::prelude::*;
use std::str::FromStr;
use std::sync::{Arc, RwLock};
use std::{
	env, fs,
	io::Write,
	path::PathBuf,
	process::{Command, Stdio},
};
use tokio::{
	io::{AsyncBufReadExt, BufReader},
	process::Command as TokioCommand,
	sync::oneshot,
	task,
};
use url::Url;

#[derive(Clone)]
pub struct SetupMovementClient {
	///The Apotos Rest Client
	pub rest_client: Client,
	///The Apotos Rest Client
	pub faucet_client: Arc<RwLock<FaucetClient>>,
	///The signer account
	pub signer: Arc<LocalAccount>,
	///Native Address of the
	pub native_address: AccountAddress,
	/// Bytes of the non-native (external) chain.
	pub non_native_address: Vec<u8>,
}

impl SetupMovementClient {
	pub async fn setup_local_movement_network(
	) -> Result<(Self, tokio::process::Child), anyhow::Error> {
		let (setup_complete_tx, setup_complete_rx) = oneshot::channel();
		let mut child = TokioCommand::new("movement")
			.args(&["node", "run-local-testnet", "--force-restart", "--assume-yes"])
			.stdout(Stdio::piped())
			.stderr(Stdio::piped())
			.spawn()?;

		let stdout = child.stdout.take().expect("Failed to capture stdout");
		let stderr = child.stderr.take().expect("Failed to capture stderr");

		task::spawn(async move {
			let mut stdout_reader = BufReader::new(stdout).lines();
			let mut stderr_reader = BufReader::new(stderr).lines();
			let mut setup_complete_tx = Some(setup_complete_tx);
			loop {
				tokio::select! {
					line = stdout_reader.next_line() => {
						match line {
							Ok(Some(line)) => {
								println!("STDOUT: {}", line);
								if line.contains("Setup is complete") {
									println!("Testnet is up and running!");
									let setup_complete_tx = setup_complete_tx.take();
									if let Some(tx) = setup_complete_tx {
										let _ = tx.send(());
									}
									// return Ok(());
								}
							},
							Ok(None) => {
								return Err::<(), _>(anyhow::anyhow!("Unexpected end of stdout stream"));
							},
							Err(e) => {
								return Err::<(), _>(anyhow::anyhow!("Error reading stdout: {}", e));
							}
						}
					},
					line = stderr_reader.next_line() => {
						match line {
							Ok(Some(line)) => {
								println!("STDERR: {}", line);
								if line.contains("Setup is complete") {
									println!("Testnet is up and running!");
									let setup_complete_tx = setup_complete_tx.take();
									if let Some(tx) = setup_complete_tx {
										let _ = tx.send(());
									}
									//  return Ok(());
								}
							},
							Ok(None) => {
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
		println!("Setup complete message received.");

		let node_connection_url = "http://127.0.0.1:8080".to_string();
		let node_connection_url = Url::from_str(node_connection_url.as_str())
			.map_err(|_| BridgeContractError::SerializationError)?;
		let rest_client = Client::new(node_connection_url.clone());

		let faucet_url = "http://127.0.0.1:8081".to_string();
		let faucet_url = Url::from_str(faucet_url.as_str())
			.map_err(|_| BridgeContractError::SerializationError)?;
		let faucet_client = Arc::new(RwLock::new(FaucetClient::new(
			faucet_url.clone(),
			node_connection_url.clone(),
		)));

		let mut address_bytes = [0u8; AccountAddress::LENGTH];
		address_bytes[0..2].copy_from_slice(&[0xca, 0xfe]);
		let native_address = AccountAddress::new(address_bytes);

		let mut rng = ::rand::rngs::StdRng::from_seed([3u8; 32]);
		Ok((
			SetupMovementClient {
				rest_client,
				faucet_client: faucet_client,
				signer: Arc::new(LocalAccount::generate(&mut rng)),
				native_address,
				non_native_address: Vec::new(),
			},
			child,
		))
	}

	pub fn publish_for_test(&mut self) -> Result<()> {
		let random_seed = rand::thread_rng().gen_range(0, 1000000).to_string();

		let mut process = Command::new("movement")
			.args(&["init"])
			.stdin(Stdio::piped())
			.stdout(Stdio::piped())
			.stderr(Stdio::piped())
			.spawn()
			.expect("Failed to execute command");

		let private_key_hex = hex::encode(self.signer.private_key().to_bytes());

		let stdin: &mut std::process::ChildStdin =
			process.stdin.as_mut().expect("Failed to open stdin");

		let movement_dir = PathBuf::from(".movement");

		if movement_dir.exists() {
			stdin.write_all(b"yes\n").expect("Failed to write to stdin");
		}

		stdin.write_all(b"local\n").expect("Failed to write to stdin");

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
		self.native_address = AccountAddress::from_hex_literal(&formatted_resource_address)?;

		println!("Publish Derived resource address: {}", formatted_resource_address);

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

	pub async fn initiator_set_timelock(&mut self, time_lock: u64) -> Result<(), anyhow::Error> {
		let args = vec![bridge_service::chains::movement::utils::serialize_u64(&time_lock)
			.expect("Failed to serialize time lock")];

		let payload = bridge_service::chains::movement::utils::make_aptos_payload(
			self.native_address,
			"atomic_bridge_initiator",
			"set_time_lock_duration",
			Vec::new(),
			args,
		);

		println!("Payload: {:?}", payload);

		bridge_service::chains::movement::utils::send_and_confirm_aptos_transaction(
			&self.rest_client,
			self.signer.as_ref(),
			payload,
		)
		.await
		.map_err(|err| anyhow::anyhow!(err))?;

		Ok(())
	}

	pub async fn movement_initiate_bridge_transfer_helper(
		&mut self,
		initiator_address: AccountAddress,
		recipient_address: Vec<u8>,
		hash_lock: [u8; 32],
		amount: u64,
		timelock_modify: bool,
	) -> Result<(), anyhow::Error> {
		if timelock_modify {
			// Set the timelock to 1 second for testing
			self.initiator_set_timelock(1).await.expect("Failed to set timelock");
		}

		// Mint MovETH to the initiator's address
		let mint_amount = 200 * 100_000_000; // Assuming 8 decimals for MovETH

		let mint_args = vec![
			bridge_service::chains::movement::utils::serialize_address_initiator(
				&self.signer.address(),
			)?, // Mint to initiator's address
			bridge_service::chains::movement::utils::serialize_u64_initiator(&mint_amount)?, // Amount to mint (200 MovETH)
		];

		let mint_payload = bridge_service::chains::movement::utils::make_aptos_payload(
			self.native_address, // Address where moveth module is published
			"moveth",
			"mint",
			Vec::new(),
			mint_args,
		);

		// Send transaction to mint MovETH
		bridge_service::chains::movement::utils::send_and_confirm_aptos_transaction(
			&self.rest_client,
			&self.signer,
			mint_payload,
		)
		.await
		.map_err(|err| anyhow::anyhow!(err))?;

		debug!("Successfully minted 200 MovETH to the initiator");

		// Initiate the bridge transfer
		self.initiate_bridge_transfer(
			BridgeAddress(MovementAddress(initiator_address)),
			BridgeAddress(recipient_address),
			HashLock(hash_lock),
			Amount(AssetType::Moveth(amount)),
		)
		.await
		.expect("Failed to initiate bridge transfer");

		Ok(())
	}
	async fn initiate_bridge_transfer(
		&mut self,
		_initiator: BridgeAddress<MovementAddress>,
		recipient: BridgeAddress<Vec<u8>>,
		hash_lock: HashLock,
		amount: Amount,
	) -> BridgeContractResult<()> {
		let amount_value = match amount.0 {
			AssetType::Moveth(value) => value,
			_ => return Err(BridgeContractError::ConversionFailed("Amount".to_string())),
		};
		debug!("Amount value: {:?}", amount_value);

		let args = vec![
			bridge_service::chains::movement::utils::serialize_vec_initiator(&recipient.0)?,
			bridge_service::chains::movement::utils::serialize_vec_initiator(&hash_lock.0[..])?,
			bridge_service::chains::movement::utils::serialize_u64_initiator(&amount_value)?,
		];

		let payload = bridge_service::chains::movement::utils::make_aptos_payload(
			self.native_address,
			"atomic_bridge_initiator",
			"initiate_bridge_transfer",
			Vec::new(),
			args,
		);

		let _ = bridge_service::chains::movement::utils::send_and_confirm_aptos_transaction(
			&self.rest_client,
			self.signer.as_ref(),
			payload,
		)
		.await
		.map_err(|_| BridgeContractError::InitiateTransferError)?;

		Ok(())
	}
}
