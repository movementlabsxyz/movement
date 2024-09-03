use crate::utils::MovementAddress;
use anyhow::Result;
use aptos_api::accounts::Account;
use aptos_sdk::{
	move_types::{identifier::Identifier, language_storage::{ModuleId, TypeTag}},
	rest_client::{Client, FaucetClient, Response},
	types::LocalAccount,
};
use aptos_api_types::{ViewFunction, ViewRequest};
use aptos_types::account_address::AccountAddress;
use bridge_shared::{
	bridge_contracts::{
		BridgeContractCounterparty, BridgeContractCounterpartyError,
		BridgeContractCounterpartyResult,
	},
	types::{
		Amount, AssetType, BridgeTransferDetails, BridgeTransferId, HashLock, HashLockPreImage, InitiatorAddress, RecipientAddress, TimeLock
	},
};
use hex::{decode, FromHex};
use rand::prelude::*;
use rand::Rng;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{env, fs, io::{Read, Write}, path::{Path, PathBuf}, process::{Command, Stdio}};
use std::str::FromStr;
use std::sync::{Arc, RwLock};
use tokio::{
	io::{AsyncBufReadExt, BufReader},
	process::Command as TokioCommand,
	sync::oneshot,
	task,
};

use url::Url;

mod types;
pub mod utils;

const DUMMY_ADDRESS: AccountAddress = AccountAddress::new([0; 32]);
const COUNTERPARTY_MODULE_NAME: &str = "atomic_bridge_counterparty";

#[allow(dead_code)]
enum Call {
	Lock,
	Complete,
	Abort,
	GetDetails,
}

pub struct Config {
	pub rpc_url: Option<String>,
	pub ws_url: Option<String>,
	pub chain_id: String,
	pub signer_private_key: Arc<RwLock<LocalAccount>>,
	pub initiator_contract: Option<MovementAddress>,
	pub gas_limit: u64,
}

impl Config {
	pub fn build_for_test() -> Self {
		let seed = [3u8; 32];
		let mut rng = rand::rngs::StdRng::from_seed(seed);

		Config {
			rpc_url: Some("http://localhost:8080".parse().unwrap()),
			ws_url: Some("ws://localhost:8080".parse().unwrap()),
			chain_id: 4.to_string(),
			signer_private_key: Arc::new(RwLock::new(LocalAccount::generate(&mut rng))),
			initiator_contract: None,
			gas_limit: 10_000_000_000,
		}
	}
}

#[allow(dead_code)]
#[derive(Clone)]
pub struct MovementClient {
	///Address of the counterparty moduke
	counterparty_address: AccountAddress,
	///Address of the initiator module
	initiator_address: Vec<u8>,
	///The Apotos Rest Client
	pub rest_client: Client,
	///The Apotos Rest Client
	pub faucet_client: Option<Arc<RwLock<FaucetClient>>>,
	///The signer account
	signer: Arc<LocalAccount>,
}

impl MovementClient {
	pub async fn new(_config: Config) -> Result<Self, anyhow::Error> {
		let node_connection_url = "http://127.0.0.1:8080".to_string();
		let node_connection_url = Url::from_str(node_connection_url.as_str()).map_err(|_| BridgeContractCounterpartyError::SerializationError)?;

		let rest_client = Client::new(node_connection_url.clone());

		let seed = [3u8; 32];
		let mut rng = rand::rngs::StdRng::from_seed(seed);
		let signer = LocalAccount::generate(&mut rng);

		let mut address_bytes = [0u8; AccountAddress::LENGTH];
        	address_bytes[0..2].copy_from_slice(&[0xca, 0xfe]);
		let counterparty_address = AccountAddress::new(address_bytes);
		Ok(MovementClient {
			counterparty_address,
			initiator_address: Vec::new(), //dummy for now
			rest_client,
			faucet_client: None,
			signer: Arc::new(signer),
		})
	}

	pub async fn new_for_test(
		_config: Config,
	) -> Result<(Self, tokio::process::Child), anyhow::Error> {
		let (setup_complete_tx, mut setup_complete_rx) = oneshot::channel();
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
							Ok(None) => {
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
		let node_connection_url = Url::from_str(node_connection_url.as_str()).map_err(|_| BridgeContractCounterpartyError::SerializationError)?;
		let rest_client = Client::new(node_connection_url.clone());

		let faucet_url = "http://127.0.0.1:8081".to_string();
		let faucet_url = Url::from_str(faucet_url.as_str()).map_err(|_| BridgeContractCounterpartyError::SerializationError)?;
		let faucet_client = Arc::new(RwLock::new(FaucetClient::new(
			faucet_url.clone(),
			node_connection_url.clone(),
		)));

		let mut rng = ::rand::rngs::StdRng::from_seed([3u8; 32]);
		Ok((
			MovementClient {
				counterparty_address: DUMMY_ADDRESS,
				initiator_address: Vec::new(), // dummy for now
				rest_client,
				faucet_client: Some(faucet_client),
				signer: Arc::new(LocalAccount::generate(&mut rng)),
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

		let stdin: &mut std::process::ChildStdin = process.stdin.as_mut().expect("Failed to open stdin");

		let movement_dir = PathBuf::from(".movement");

		if movement_dir.exists() {
			stdin.write_all(b"yes\n").expect("Failed to write to stdin");
		}

		stdin.write_all(b"local\n").expect("Failed to write to stdin");

		let _ = stdin.write_all(format!("{}\n", private_key_hex).as_bytes());

		drop(stdin);

		let addr_output = process
			.wait_with_output()
			.expect("Failed to read command output");

		if !addr_output.stdout.is_empty() {
			println!("stdout: {}", String::from_utf8_lossy(&addr_output.stdout));
		}
	
		if !addr_output.stderr.is_empty() {
			eprintln!("stderr: {}", String::from_utf8_lossy(&addr_output.stderr));
		}
		let addr_output_str = String::from_utf8_lossy(&addr_output.stderr);
		let address = addr_output_str
			.split_whitespace()
			.find(|word| word.starts_with("0x")
		) 
		    	.expect("Failed to extract the Movement account address");
	    
		println!("Extracted address: {}", address);

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
			println!("stdout: {}", String::from_utf8_lossy(&resource_output.stdout));
		}
		if !resource_output.stderr.is_empty() {
			eprintln!("stderr: {}", String::from_utf8_lossy(&resource_output.stderr));
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
		self.counterparty_address = AccountAddress::from_hex_literal(&formatted_resource_address)?;


		println!("Derived resource address: {}", formatted_resource_address);

		let current_dir = env::current_dir().expect("Failed to get current directory");
		println!("Current directory: {:?}", current_dir);

		let move_toml_path = PathBuf::from("../move-modules/Move.toml");

		// Read the existing content of Move.toml
		let move_toml_content = fs::read_to_string(&move_toml_path)
			.expect("Failed to read Move.toml file");
	
		// Update the content of Move.toml with the new addresses
		let updated_content = move_toml_content
			.lines()
			.map(|line| {
				match line {
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
				}
			})
			.collect::<Vec<_>>()
			.join("\n");
	
		// Write the updated content back to Move.toml
		let mut file = fs::File::create(&move_toml_path)
			.expect("Failed to open Move.toml file for writing");
		file.write_all(updated_content.as_bytes())
			.expect("Failed to write updated Move.toml file");
	
		println!("Move.toml updated successfully.");

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
				"../move-modules"
			])
			.stdout(Stdio::piped())
			.stderr(Stdio::piped())
			.output()
			.expect("Failed to execute command");

	if !output2.stdout.is_empty() {
		eprintln!("stdout: {}", String::from_utf8_lossy(&output2.stdout));
		}

	if !output2.stderr.is_empty() {
        	eprintln!("stderr: {}", String::from_utf8_lossy(&output2.stderr));
    	}

	
	if movement_dir.exists() {
		fs::remove_dir_all(movement_dir).expect("Failed to delete .movement directory");
		println!(".movement directory deleted successfully.");
	}

	    // Read the existing content of Move.toml
	let move_toml_content = fs::read_to_string(&move_toml_path)
	    .expect("Failed to read Move.toml file");
    
	// Directly assign the address
	let final_address = "0xcafe";

	// Directly assign the formatted resource address
	let final_formatted_resource_address = "0xc3bb8488ab1a5815a9d543d7e41b0e0df46a7396f89b22821f07a4362f75ddc5";
	
	let updated_content = move_toml_content
	    .lines()
	    .map(|line| {
		match line {
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
		}
	    })
	    .collect::<Vec<_>>()
	    .join("\n");
    
	// Write the updated content back to Move.toml
	let mut file = fs::File::create(&move_toml_path)
	    .expect("Failed to open Move.toml file for writing");
	file.write_all(updated_content.as_bytes())
	    .expect("Failed to write updated Move.toml file");
    
	println!("Move.toml addresses updated successfully at the end of the test.");    

	Ok(())
	}
	
	pub fn rest_client(&self) -> &Client {
		&self.rest_client
	}
	
	pub fn signer(&self) -> &LocalAccount {
		&self.signer
	}

	pub fn faucet_client(&self) -> Result<&Arc<RwLock<FaucetClient>>> {
		if let Some(faucet_client) = &self.faucet_client {
			Ok(faucet_client)
		} else {
			Err(anyhow::anyhow!("Faucet client not initialized"))
		}
	}
}

#[async_trait::async_trait]
impl BridgeContractCounterparty for MovementClient {
	type Address = MovementAddress;
	type Hash = [u8; 32];

	async fn lock_bridge_transfer(
		&mut self,
		bridge_transfer_id: BridgeTransferId<Self::Hash>,
		hash_lock: HashLock<Self::Hash>,
		time_lock: TimeLock,
		initiator: InitiatorAddress<Vec<u8>>,
		recipient: RecipientAddress<Self::Address>,
		amount: Amount,
	) -> BridgeContractCounterpartyResult<()> {

		let amount_value = match amount.0 {
			AssetType::Moveth(value) => value,
			_ => return Err(BridgeContractCounterpartyError::SerializationError),
		};

		let args = vec![
			bcs::to_bytes(&initiator.0).map_err(|_| BridgeContractCounterpartyError::SerializationError)?,
			bcs::to_bytes(&bridge_transfer_id.0[..]).map_err(|_| BridgeContractCounterpartyError::SerializationError)?,
			bcs::to_bytes(&hash_lock.0[..]).map_err(|_| BridgeContractCounterpartyError::SerializationError)?,
			bcs::to_bytes(&time_lock.0).map_err(|_| BridgeContractCounterpartyError::SerializationError)?,
			bcs::to_bytes(&recipient.0.0).map_err(|_| BridgeContractCounterpartyError::SerializationError)?,
			bcs::to_bytes(&amount_value).map_err(|_| BridgeContractCounterpartyError::SerializationError)?,
		];

		let payload = utils::make_aptos_payload(
			self.counterparty_address,
			COUNTERPARTY_MODULE_NAME,
			"lock_bridge_transfer",
			Vec::new(),
			args,
		);

		let result = utils::send_and_confirm_aptos_transaction(
			&self.rest_client,
			self.signer.as_ref(),
			payload,
		)
		.await
		.map_err(|_| BridgeContractCounterpartyError::LockTransferError);

		Ok(())
	}

	async fn complete_bridge_transfer(
		&mut self,
		bridge_transfer_id: BridgeTransferId<Self::Hash>,
		preimage: HashLockPreImage,
	) -> BridgeContractCounterpartyResult<()> {
		let args2 = vec![
			utils::serialize_vec(&bridge_transfer_id.0[..])?,
			utils::serialize_vec(&preimage.0)?,
		];

		let payload = utils::make_aptos_payload(
			self.counterparty_address,
			COUNTERPARTY_MODULE_NAME,
			"complete_bridge_transfer",
			Vec::new(),
			args2,
		);

		let result = utils::send_and_confirm_aptos_transaction(
			&self.rest_client,
			self.signer.as_ref(),
			payload,
		)
		.await
		.map_err(|_| BridgeContractCounterpartyError::CompleteTransferError);

		println!("Complete bridge transfer result: {:?}", &result);

		Ok(())
	}

	async fn abort_bridge_transfer(
		&mut self,
		bridge_transfer_id: BridgeTransferId<Self::Hash>,
	) -> BridgeContractCounterpartyResult<()> {
		let args = vec![
			utils::serialize_vec(&bridge_transfer_id.0)?,
		];
		let payload = utils::make_aptos_payload(
			self.counterparty_address,
			COUNTERPARTY_MODULE_NAME,
			"abort_bridge_transfer",
			Vec::new(),
			args,
		);
		let result = utils::send_and_confirm_aptos_transaction(
			&self.rest_client,
			self.signer.as_ref(),
			payload,
		)
		.await
		.map_err(|_| BridgeContractCounterpartyError::AbortTransferError);

		println!("Abort bridge transfer result: {:?}", &result);
		Ok(())
	}

	async fn get_bridge_transfer_details(
		&mut self,
		bridge_transfer_id: BridgeTransferId<[u8; 32]>,
	) -> Result<Option<BridgeTransferDetails<MovementAddress, [u8; 32]>>, BridgeContractCounterpartyError> {
		// Construct the ViewRequest
		let view_request = ViewFunction {
			module: ModuleId::new(
			self.counterparty_address.clone(), // Assuming counterparty_address is of type AccountAddress
			Identifier::new("atomic_bridge_counterparty")
				.map_err(|_| BridgeContractCounterpartyError::ModuleViewError)?,
			),
			function: Identifier::new("bridge_transfers")
			.map_err(|_| BridgeContractCounterpartyError::FunctionViewError)?,
			ty_args: vec![],
			args: vec![bcs::to_bytes(&bridge_transfer_id.0)
			.map_err(|_| BridgeContractCounterpartyError::SerializationError)?],
		};
		
		// Send the request to the "/view" endpoint using view_bcs
		let response: Response<(String, String, u64, String, u64, u8)> = self.rest_client
			.view_bcs(&view_request, None)
			.await
			.map_err(|_| BridgeContractCounterpartyError::SerializationError)?;
		
		// Check if the response is valid and parse it
		let (originator, recipient, amount, hash_lock, time_lock, state) = response.into_inner();
		
		// Convert originator and recipient addresses from hex strings to AccountAddress
		let originator_address = AccountAddress::from_hex_literal(&originator)
			.map_err(|_| BridgeContractCounterpartyError::SerializationError)?;
		let recipient_address_bytes = hex::decode(&recipient[2..])
			.map_err(|_| BridgeContractCounterpartyError::SerializationError)?;
		
		// Convert hash_lock from hex string to [u8; 32]
		let hash_lock_array: [u8; 32] = hex::decode(&hash_lock[2..])
			.map_err(|_| BridgeContractCounterpartyError::SerializationError)?
			.try_into()
			.map_err(|_| BridgeContractCounterpartyError::SerializationError)?;
		
		// Create the BridgeTransferDetails struct
		let details: BridgeTransferDetails<MovementAddress, [u8; 32]> = BridgeTransferDetails {
			bridge_transfer_id,
			initiator_address: InitiatorAddress(MovementAddress(originator_address)),
			recipient_address: RecipientAddress(recipient_address_bytes),
			amount: Amount(AssetType::Moveth(amount)),
			hash_lock: HashLock(hash_lock_array),
			time_lock: TimeLock(time_lock),
			state,
		};
		
		Ok(Some(details))
	}



	async fn get_bridge_transfer_state(
		&mut self,
		bridge_transfer_id: BridgeTransferId<[u8; 32]>,
	) -> Result<Option<u8>, BridgeContractCounterpartyError> {
	
		#[derive(Debug, Deserialize, Serialize)]
		pub struct BridgeTransfer {
			pub originator: Vec<u8>, // Eth address as bytes
			pub recipient: String,   // Account address
			pub amount: u64,
			pub hash_lock: Vec<u8>,
			pub time_lock: u64,
			pub state: u8,
		}
	
		#[derive(Debug, Deserialize, Serialize)]
		pub struct BridgeTransferStore {
			pub transfers: Vec<BridgeTransfer>,
			// Include other fields if necessary
		}
	
		// Construct the resource path to BridgeTransferStore
		let resource_path = format!(
			"{}::atomic_bridge_counterparty::BridgeTransferStore",
			self.counterparty_address.to_string(),
		);
	
		// Query the BridgeTransferStore resource
		let response = self.rest_client
			.get_account_resource(self.counterparty_address, &resource_path)
			.await
			.map_err(|e| {
				tracing::error!("Failed to get account resource: {:?}", e);
				BridgeContractCounterpartyError::ViewSerializationError
			})?;
	
		// Handle the Option<Resource>
		let resource = match response.into_inner() {
			Some(resource) => resource,
			None => {
				tracing::error!("No resource found at the given path.");
				return Ok(None); // Return None if the resource is not found
			}
		};
		tracing::info!("Resource: {:?}", resource);

		// Access the 'transfers' handle directly from the resource data
		let transfers_handle = resource
		.data
		.get("transfers")
		.and_then(|transfers| transfers.get("buckets"))
		.and_then(|buckets| buckets.get("inner"))
		.and_then(|inner| inner.get("handle"))
		.and_then(|handle| handle.as_str())
		.ok_or_else(|| {
		tracing::error!("Failed to extract transfers handle from resource data.");
		BridgeContractCounterpartyError::SerializationError
		})?;

		tracing::info!("Transfers handle: {:?}", transfers_handle);
	
		// Convert the bridge_transfer_id to a hex string for the lookup
		let id_hex = format!("0x{}", hex::encode(bridge_transfer_id.0));

		tracing::info!("Id hex: {:?}", id_hex);
		
		let mut transfers_handle_cleaned = transfers_handle.trim().trim_start_matches("0x").to_string();

		if transfers_handle_cleaned.len() == 63 {
		    // Prepend a leading zero to make the length 64 characters
		    transfers_handle_cleaned = format!("0{}", transfers_handle_cleaned);
		}
		
		tracing::info!("Transfers handle after fixing: {:?}, length: {}", transfers_handle_cleaned, transfers_handle_cleaned.len());
		
		let transfers_handle_bytes = decode(transfers_handle_cleaned).map_err(|e| {
		    tracing::error!("Failed to decode transfers handle: {:?}", e);
		    BridgeContractCounterpartyError::SerializationError
		})?;
	    
		if transfers_handle_bytes.len() != 32 {
		    tracing::error!("Transfers handle has an incorrect length: {}", transfers_handle_bytes.len());
		    return Err(BridgeContractCounterpartyError::SerializationError);
		}
	    
		let mut address_bytes = [0u8; 32];
		address_bytes.copy_from_slice(&transfers_handle_bytes);
	    
		let transfers_handle_address = AccountAddress::new(address_bytes);
	    
		tracing::info!("Transfers handle as AccountAddress: {:?}", transfers_handle_address);	    

		// Query the specific BridgeTransfer using the transfers handle and the ID
		let transfer_response = self.rest_client
			.get_table_item(
				transfers_handle_address,
				"vector<u8>",
				&format!("{}::atomic_bridge_counterparty::BridgeTransfer", self.counterparty_address),
				id_hex,
			)
			.await
			.map_err(|e| {
			tracing::error!("Failed to get transfer item: {:?}", e);
			BridgeContractCounterpartyError::ViewSerializationError
			})?;

		let transfer_value = transfer_response.into_inner();
		
		// Deserialize the specific transfer into the BridgeTransfer struct
		let bridge_transfer: BridgeTransfer = serde_json::from_value(transfer_value)
			.map_err(|e| {
			tracing::error!("Deserialization error for transfer: {:?}", e);
			BridgeContractCounterpartyError::SerializationError
			})?;
		
		// Return the state of the bridge transfer or None if not found
		Ok(Some(bridge_transfer.state))
	}
	
}

impl MovementClient {
	fn counterparty_type_args(&self, call: Call) -> Vec<TypeTag> {
		match call {
			Call::Lock => vec![TypeTag::Address, TypeTag::U64, TypeTag::U64, TypeTag::U8],
			Call::Complete => vec![TypeTag::Address, TypeTag::U64, TypeTag::U8],
			Call::Abort => vec![TypeTag::Address, TypeTag::U64],
			Call::GetDetails => vec![TypeTag::Address, TypeTag::U64],
		}
	}
}

