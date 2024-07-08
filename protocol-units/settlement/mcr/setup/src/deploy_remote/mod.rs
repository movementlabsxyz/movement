use super::Setup;
use anyhow::anyhow;
use commander::run_command;
use dot_movement::DotMovement;
use mcr_settlement_config::Config;
use serde_json::Value;
use alloy_signer_wallet::LocalWallet;

use std::future::Future;
use tracing::info;

/// The local setup strategy for MCR settlement
#[derive(Debug, Clone)]
pub struct DeployRemote {}

impl DeployRemote {
	/// Instantiates the local setup strategy with ports on localhost
	/// to configure for Ethernet RPC and WebSocket client access.
	pub fn new() -> Self {
		Self { }
	}
}

impl Default for DeployRemote {
	fn default() -> Self {
		DeployRemote::new()
	}
}

impl Setup for DeployRemote {
	fn setup(
		&self,
		dot_movement: &DotMovement,
		mut config: Config,
	) -> impl Future<Output = Result<(Config, tokio::task::JoinHandle<Result<String, anyhow::Error>>), anyhow::Error>> + Send {

		tracing::info!("Init Settlement local conf");

		async move {

			tracing::info!("Run Settlement local conf: {:?}", config.signer_private_key);
			
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
					"--sender",
					&config.try_governor_address()?.to_string(),
					"--rpc-url",
					&config.eth_rpc_connection_url(),
					"--private-key",
					&config.governor_private_key,
					"--legacy"
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
			info!("DeployRemotement JSON value: {json_value:#?}");

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

            // generate random well-known accounts and addresses
            for _ in 0..10 {
                let wallet = LocalWallet::random();
                config.well_known_accounts.push(wallet.to_bytes().to_string());
                config.well_known_addresses.push(wallet.address().to_string());
            }

            let join_handle = tokio::spawn(async {
                // Create a future that waits forever
                std::future::pending::<Result<String, anyhow::Error>>().await
            });

			Ok((config, join_handle))
		}
	}
}
