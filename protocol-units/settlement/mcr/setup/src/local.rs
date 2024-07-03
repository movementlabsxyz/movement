use super::Setup;
use anyhow::anyhow;
use commander::{run_command, spawn_command};
use dot_movement::DotMovement;
use mcr_settlement_config::Config;
use rand::{thread_rng, Rng};
use serde_json::Value;

use std::future::Future;
use tracing::info;

const DEFAULT_ETH_RPC_PORT: u16 = 8545;
const DEFAULT_ETH_WS_PORT: u16 = 8545;

/// The local setup strategy for MCR settlement
#[derive(Debug, Clone)]
pub struct Local {
	eth_rpc_port: u16,
	eth_ws_port: u16,
}

impl Local {
	/// Instantiates the local setup strategy with ports on localhost
	/// to configure for Ethernet RPC and WebSocket client access.
	pub fn new(eth_rpc_port: u16, eth_ws_port: u16) -> Self {
		Self { eth_rpc_port, eth_ws_port }
	}
}

impl Default for Local {
	fn default() -> Self {
		Local::new(DEFAULT_ETH_RPC_PORT, DEFAULT_ETH_WS_PORT)
	}
}

impl Setup for Local {
	fn setup(
		&self,
		dot_movement: &DotMovement,
		mut config: Config,
	) -> impl Future<Output = Result<Config, anyhow::Error>> + Send {
		// Define a temporary chain Id for Anvil
		let mut rng = thread_rng(); // rng is not send.
		let id: u16 = rng.gen_range(100, 32768);
		let chain_id = id.to_string();

		tracing::info!("Init Settlement local conf");

		async move {
			if config.rpc_url.is_none() {
				config.rpc_url = Some(format!("http://localhost:{}", self.eth_rpc_port));
			}
			if config.ws_url.is_none() {
				config.ws_url = Some(format!("ws://localhost:{}", self.eth_ws_port));
			}

			tracing::info!("Run Settlement local conf: {:?}", config.signer_private_key);
			if config.signer_private_key.is_none() {
				// Start the local process and deploy the smart contract.
				// Define the working directory of Anvil
				let mut path = dot_movement.get_path().to_path_buf();
				path.push("anvil/mcr");
				path.push(chain_id.clone());
				tokio::fs::create_dir_all(&path).await?;
				path.push("anvil.json");

				let anvil_path = path.to_string_lossy().to_string();

				let (anvil_cmd_id, _jh) = spawn_command(
					"anvil".to_string(),
					vec![
						"--chain-id".to_string(),
						chain_id.clone(),
						"--config-out".to_string(),
						anvil_path.clone(),
						"--port".to_string(),
						DEFAULT_ETH_RPC_PORT.to_string(),
					],
				)
				.await?;
				// Wait Anvil to start
				let mut counter = 0;
				loop {
					if counter > 10 {
						return Err(anyhow!("Anvil doesn't start in time"));
					}
					counter += 1;
					if path.exists() {
						break;
					}
					let _ = tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
				}

				// Load Anvil Conf
				let mut anvil_conf = mcr_settlement_config::anvil::TestLocal::new(&path)?;

				// Deploy MCR smart contract.
				// Remove the settlement key from the Anvil keys to avoid its reuse.
				let smart_contract_key = anvil_conf.anvil_keys.remove(0);
				let smart_contract_private_key = smart_contract_key.private_key;
				let smart_contract_address = smart_contract_key.address;

				let mut solidity_path = std::env::current_dir()?;
				solidity_path.push("protocol-units/settlement/mcr/contracts");

				let solidity_path = solidity_path.to_string_lossy();
				tracing::info!("solidity_path: {:?}", solidity_path);
				let output_exec = run_command(
					"forge",
					&[
						"script",
						"DeployMCRLegacy",
						"--root",
						&solidity_path,
						"--broadcast",
						"--chain-id",
						&chain_id,
						"--sender",
						&smart_contract_address,
						"--rpc-url",
						config.rpc_url.as_ref().unwrap(),
						"--private-key",
						&smart_contract_private_key,
					],
				)
				.await?
				.trim()
				.to_string();

				// Get the summary execution file path from the output;
				let line = output_exec
					.lines()
					.find(|line| line.contains("Transactions saved to:"))
					.ok_or(anyhow!(
						"Can't file exec file path in smart contract deployement result output."
					))?;
				let path = line
					.splitn(2, ':')
					.nth(1)
					.ok_or(anyhow!(
					"No path after 'Transactions saved to:' in smart contract deployement result output."
				))?
					.trim();
				// Read the summary to get the contract address
				let json_text = std::fs::read_to_string(path)?;
				// Get the value of the field contractAddress under the transactions array
				let json_value: Value =
					serde_json::from_str(&json_text).expect("Error parsing JSON");

				// Extract the contract address
				let mcr_address = json_value["transactions"]
					.as_array()
					.and_then(|transactions| transactions.get(0))
					.and_then(|transaction| transaction.as_object())
					.and_then(|transaction_object| transaction_object.get("contractAddress"))
					.ok_or(anyhow!(
						"No contract address in forge script exec deployement result file."
					))
					.map(|v| {
						let s = v.as_str().expect("Contract address elements should be strings");
						s.to_owned()
					})?;

				info!("setting up MCR Ethereum client mcr_address:{mcr_address}");
				// The First address in the key list is the one used by the settlement client and genesis ceremonial.
				config.signer_private_key = Some(anvil_conf.anvil_keys[0].private_key.clone());
				config.mcr_contract_address = mcr_address;
				config.anvil_process_pid = anvil_cmd_id;
				config.test_local = Some(anvil_conf);

				info!("MCR config:{config:?}");
			}

			Ok(config)
		}
	}
}
