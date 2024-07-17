use alloy::signers::local::PrivateKeySigner;
use anyhow::anyhow;
use anyhow::Context;
use commander::run_command;
use dot_movement::DotMovement;
use mcr_settlement_config::{common, Config};
use serde_json::Value;
use tracing::info;

/// The local setup strategy for MCR settlement
#[derive(Debug, Clone)]
pub struct Deploy {}

impl Deploy {
	/// Instantiates the local setup strategy with ports on localhost
	/// to configure for Ethernet RPC and WebSocket client access.
	pub fn new() -> Self {
		Self {}
	}
}

impl Default for Deploy {
	fn default() -> Self {
		Deploy::new()
	}
}

impl Deploy {
	pub async fn setup(
		&self,
		dot_movement: &DotMovement,
		mut config: Config,
		deploy: &common::deploy::Config,
	) -> Result<Config, anyhow::Error> {
		// enforce config.deploy = deploy
		config.deploy = Some(deploy.clone());

		let wallet: PrivateKeySigner = deploy.mcr_deployment_account_private_key.parse()?;

		// todo: make sure this shows up in the docker container as well
		let mut solidity_path = std::env::current_dir()?;
		solidity_path.push(deploy.mcr_deployment_working_directory.clone());

		let solc_path = run_command("which", &["solc"])
			.await
			.context("Failed to get solc path")?
			.trim()
			.to_string();

		let solidity_path = solidity_path.to_string_lossy();
		tracing::info!("solidity_path: {:?}", solidity_path);
		run_command("forge", &["compile", "--root", &solidity_path, "--use", &solc_path])
			.await
			.context("Failed to compile with MCR workspace")?;

		let output_exec = run_command(
			"forge",
			&[
				"script",
				"DeployMCRDev",
				"--root",
				&solidity_path,
				"--broadcast",
				"--sender",
				&wallet.address().to_string(),
				"--rpc-url",
				&config.eth_rpc_connection_url(),
				"--private-key",
				&deploy.mcr_deployment_account_private_key,
				"--legacy",
				"--use",
				&solc_path,
			],
		)
		.await?
		.trim()
		.to_string();

		println!("DeployMCRDev output_exec:{output_exec:?}",);

		//get the summary execution file path from output;
		let line = output_exec.lines().find(|line| line.contains("Transactions saved to:")).ok_or(
			anyhow!("Can't file exec file path in smart contract deployment result output."),
		)?;
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
		let json_value: Value = serde_json::from_str(&json_text).expect("Error parsing JSON");
		info!("Deployment JSON value: {json_value:#?}");

		// Extract the move token contract address
		let move_token_address = json_value["transactions"]
			.as_array()
			.and_then(|transactions| transactions.get(3))
			.and_then(|transaction| transaction.as_object())
			.and_then(|transaction_object| transaction_object.get("contractAddress"))
			.ok_or(anyhow!("No contract address in forge script exec deployment result file."))
			.map(|v| {
				let s = v.as_str().expect("Contract address elements should be strings");
				s.to_owned()
			})?;

		// Extract the movement staking contract address
		let movement_staking_address = json_value["transactions"]
			.as_array()
			.and_then(|transactions| transactions.get(4))
			.and_then(|transaction| transaction.as_object())
			.and_then(|transaction_object| transaction_object.get("contractAddress"))
			.ok_or(anyhow!("No contract address in forge script exec deployment result file."))
			.map(|v| {
				let s = v.as_str().expect("Contract address elements should be strings");
				s.to_owned()
			})?;

		// Extract the contract address
		let mcr_address = json_value["transactions"]
			.as_array()
			.and_then(|transactions| transactions.get(5))
			.and_then(|transaction| transaction.as_object())
			.and_then(|transaction_object| transaction_object.get("contractAddress"))
			.ok_or(anyhow!("No contract address in forge script exec deployment result file."))
			.map(|v| {
				let s = v.as_str().expect("Contract address elements should be strings");
				s.to_owned()
			})?;

		// generate random well-known accounts and addresses
		let mut well_known_account_private_keys =
			if let Some(existing_testing_config) = config.testing.clone() {
				existing_testing_config.well_known_account_private_keys
			} else {
				let mut keys = Vec::new();
				for _ in 0..10 {
					let wallet = PrivateKeySigner::random();
					keys.push(wallet.to_bytes().to_string());
				}
				keys
			};

		info!("setting up MCR Ethereum client move_token_address: {move_token_address}");
		info!(
			"setting up MCR Ethereum client movement_staking_address: {movement_staking_address}"
		);
		info!("setting up MCR Ethereum client mcr_address: {mcr_address}");
		let testing_config = common::testing::Config {
			well_known_account_private_keys: well_known_account_private_keys,
			mcr_testing_admin_account_private_key: deploy
				.mcr_deployment_account_private_key
				.clone(),
			move_token_contract_address: move_token_address,
			movement_staking_contract_address: movement_staking_address,
		};

		config.settle.mcr_contract_address = mcr_address;

		Ok(config)
	}
}
