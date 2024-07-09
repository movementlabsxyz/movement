use super::Setup;
use anyhow::anyhow;
use commander::{run_command, spawn_command};
use dot_movement::DotMovement;
use mcr_settlement_config::Config;
use rand::{thread_rng, Rng};
use serde_json::Value;

use std::future::Future;
use tracing::info;

/// The local setup strategy for MCR settlement
#[derive(Debug, Clone)]
pub struct Local {}

impl Local {
	/// Instantiates the local setup strategy with ports on localhost
	/// to configure for Ethernet RPC and WebSocket client access.
	pub fn new() -> Self {
		Self { }
	}
}

impl Default for Local {
	fn default() -> Self {
		Local::new()
	}
}

impl Setup for Local {
	fn setup(
		&self,
		dot_movement: &DotMovement,
		mut config: Config,
	) -> impl Future<Output = Result<(Config, tokio::task::JoinHandle<Result<String, anyhow::Error>>), anyhow::Error>> + Send {
		//define a temporary chain Id for Anvil
		let mut rng = thread_rng(); // rng is not send.
		let id: u16 = rng.gen_range(100, 32768);
		let chain_id = id.to_string();
		config.eth_chain_id = id as u64;

		tracing::info!("Init Settlement local conf");

		async move {
			
			//start local process and deploy smart contract.
			//define working directory of Anvil
			let mut path = dot_movement.get_path().to_path_buf();
			path.push("anvil/mcr");
			path.push(chain_id.clone());
			tokio::fs::create_dir_all(&path).await?;
			path.push("anvil.json");

			let anvil_path = path.to_string_lossy().to_string();

			let (anvil_cmd_id, anvil_join_handle) = spawn_command(
				"anvil".to_string(),
				vec![
					"--chain-id".to_string(),
					config.eth_chain_id.to_string(),
					"--config-out".to_string(),
					anvil_path.clone(),
					"--port".to_string(),
					config.eth_rpc_connection_port.to_string(),
					"--steps-tracing".to_string()
				],
			)
			.await?;
			//wait Anvil to start
			let mut counter = 0;
			loop {
				if counter > 10 {
					return Err(anyhow!("Anvil didn't start in time"));
				}
				counter += 1;
				if path.exists() {
					break;
				}
				let _ = tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
			}

			// Deploy MCR smart contract.
			let anvil_addresses =
				mcr_settlement_client::eth_client::read_anvil_json_file_addresses(
					&*anvil_path,
				)?;
			config.governor_private_key = anvil_addresses.get(0).ok_or(
				anyhow!("Governor private key not found in Anvil addresses"),
			)?.private_key.clone();

			// set the signer private key to the governor private key 
			// so that it can mint for itself in future iterations of local mode testing
			config.signer_private_key = config.governor_private_key.clone();

			let governor_address = anvil_addresses.get(0).ok_or(
				anyhow!("Governor address not found in Anvil addresses"),
			)?.address.clone();

			// todo: make sure this shows up in the docker container as well
			let mut solidity_path = std::env::current_dir()?;
			solidity_path.push("protocol-units/settlement/mcr/contracts");

			let solidity_path = solidity_path.to_string_lossy();
			tracing::info!("solidity_path: {:?}", solidity_path);
			let output_exec = run_command(
				"forge",
				&[
					"script",
					"DeployMCRDev",
					"--root",
					&solidity_path,
					"--broadcast",
					"--chain-id",
					&config.eth_chain_id.to_string(),
					"--sender",
					&governor_address,
					"--rpc-url",
					&config.eth_rpc_connection_url(),
					"--private-key",
					&config.governor_private_key,
				],
			)
			.await?
			.trim()
			.to_string();

			//get the summary execution file path from output;
			let line = output_exec
				.lines()
				.find(|line| line.contains("Transactions saved to:"))
				.ok_or(anyhow!(
					"Can't file exec file path in smart contract deployment result output."
				))?;
			let path = line
				.splitn(2, ':')
				.nth(1)
				.ok_or(anyhow!(
				"No path after 'Transactions saved to:' in smart contract deployment result output."
			))?
				.trim();
			//read the summary to get the contract address
			let json_text = std::fs::read_to_string(path)?;
			//Get the value of the field contractAddress under transactions array
			let json_value: Value =
				serde_json::from_str(&json_text).expect("Error parsing JSON");
			info!("Deployment JSON value: {json_value:#?}");

			// Extract the move token contract address
			let move_token_address = json_value["transactions"]
				.as_array()
				.and_then(|transactions| transactions.get(3))
				.and_then(|transaction| transaction.as_object())
				.and_then(|transaction_object| transaction_object.get("contractAddress"))
				.ok_or(anyhow!(
					"No contract address in forge script exec deployment result file."
				))
				.map(|v| {
					let s = v.as_str().expect("Contract address elements should be strings");
					s.to_owned()
				})?;
			info!("setting up MCR Ethereum client move_token_address: {move_token_address}");
			config.move_token_contract_address = move_token_address.to_string();
			
			// Extract the movement staking contract address
			let movement_staking_address = json_value["transactions"]
				.as_array()
				.and_then(|transactions| transactions.get(4))
				.and_then(|transaction| transaction.as_object())
				.and_then(|transaction_object| transaction_object.get("contractAddress"))
				.ok_or(anyhow!(
					"No contract address in forge script exec deployment result file."
				))
				.map(|v| {
					let s = v.as_str().expect("Contract address elements should be strings");
					s.to_owned()
				})?;
			info!("setting up MCR Ethereum client movement_staking_address: {movement_staking_address}");
			config.movement_staking_contract_address = movement_staking_address.to_string();

			// Extract the contract address
			let mcr_address = json_value["transactions"]
				.as_array()
				.and_then(|transactions| transactions.get(5))
				.and_then(|transaction| transaction.as_object())
				.and_then(|transaction_object| transaction_object.get("contractAddress"))
				.ok_or(anyhow!(
					"No contract address in forge script exec deployment result file."
				))
				.map(|v| {
					let s = v.as_str().expect("Contract address elements should be strings");
					s.to_owned()
				})?;
			info!("setting up MCR Ethereum client mcr_address: {mcr_address}");
			config.mcr_contract_address = mcr_address.to_string();

			config.well_known_accounts = anvil_addresses
				.iter()
				.map(|account| account.private_key.clone())
				.collect();
			info!("MCR config:{config:?}");

			config.well_known_addresses = anvil_addresses
				.iter()
				.map(|account| account.address.clone())
				.collect();

			Ok((config, anvil_join_handle))
		}
	}
}
