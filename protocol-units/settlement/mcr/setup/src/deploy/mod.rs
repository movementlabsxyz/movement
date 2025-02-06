use alloy::providers::Provider;
use alloy::providers::ProviderBuilder;
use alloy::signers::Signer;
use alloy_network::EthereumWallet;
use alloy_network::TransactionBuilder;
use alloy_primitives::U256;
use anyhow::anyhow;
use anyhow::Context;
use commander::run_command;
use dot_movement::DotMovement;
use mcr_settlement_client::eth_client::MCR;
use mcr_settlement_config::{common, Config};
use movement_signer::cryptography::secp256k1::Secp256k1;
use movement_signer_aws_kms::hsm::AwsKms;
use movement_signer_loader::identifiers::SignerIdentifier;
use movement_signing_eth::HsmSigner;
use serde_json::Value;
use std::str::FromStr;
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
		_dot_movement: &DotMovement,
		mut config: Config,
		deploy: &common::deploy::Config,
	) -> Result<Config, anyhow::Error> {
		// enforce config.deploy = deploy
		config.deploy = Some(deploy.clone());
		let raw_private_key = deploy.signer_identifier.try_raw_private_key()?;
		let hex_string = hex::encode(raw_private_key);
		let wallet: PrivateKeySigner = hex_string.parse()?;

		// todo: make sure this shows up in the docker container as well
		let mut solidity_path = std::env::current_dir()?;
		solidity_path.push(deploy.mcr_deployment_working_directory.clone());

		// Define Foundry config file.
		let mut sol_config_path = solidity_path.clone();
		sol_config_path.push("foundry.toml");
		let sol_config_path = sol_config_path.to_string_lossy();

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

		// get the raw private key
		let raw_private_key = deploy.signer_identifier.try_raw_private_key()?;
		let hex_string = hex::encode(raw_private_key);

		let output_exec = run_command(
			"forge",
			&[
				"script",
				"DeployMCRDev",
				"--root",
				&solidity_path,
				"--config-path",
				&sol_config_path,
				"--broadcast",
				"--sender",
				&wallet.address().to_string(),
				"--rpc-url",
				&config.eth_rpc_connection_url(),
				"--private-key",
				&hex_string,
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

		info!("setting up MCR Ethereum client move_token_address: {move_token_address}");
		info!(
			"setting up MCR Ethereum client movement_staking_address: {movement_staking_address}"
		);
		info!("setting up MCR Ethereum client mcr_address: {mcr_address}");

		if let Some(testing) = &mut config.testing {
			// get the raw private key
			let raw_private_key = deploy.signer_identifier.try_raw_private_key()?;
			let hex_string = hex::encode(raw_private_key);

			testing.mcr_testing_admin_account_private_key = hex_string;
			testing.move_token_contract_address = move_token_address;
			testing.movement_staking_contract_address = movement_staking_address;
		}

		use alloy::rpc::types::TransactionRequest;
		use alloy::signers::local::PrivateKeySigner;

		// Manage signer contract role update
		// For Local signer the deployment account is used so no need to update.
		// For Was signer the AWS key account must be declared has a TrustedAttester.
		match config.settle.signer_identifier {
			SignerIdentifier::Local(_) => (),
			SignerIdentifier::AwsKms(ref aws) => {
				let key_id = aws.key.key_name();
				let aws: AwsKms<Secp256k1> =
					AwsKms::try_from_env_with_key(key_id.to_string()).await?;
				let signer =
					HsmSigner::try_new(aws, Some(config.eth_connection.eth_chain_id)).await?;
				let address = signer.address();

				let rpc_url = config.eth_rpc_connection_url();

				// get the raw private key
				let raw_private_key = deploy.signer_identifier.try_raw_private_key()?;
				let hex_string = hex::encode(raw_private_key);

				let admin = PrivateKeySigner::from_str(&hex_string)?;
				let admin_address = admin.address();
				let admin_provider = ProviderBuilder::new()
					.with_recommended_fillers()
					.wallet(EthereumWallet::new(admin))
					.on_builtin(&rpc_url.to_string())
					.await?;
				// Grant Attester role to AWS account.
				let mcr_contract = MCR::new(mcr_address.parse()?, &admin_provider);
				let grant_attester_call =
					mcr_contract.grantTrustedAttester(address).from(admin_address);
				grant_attester_call.send().await?.get_receipt().await?;

				//transfer some fund to the AWS key account
				let tx = TransactionRequest::default()
					.with_to(address)
					.with_value(U256::from(100_000_000_000_000_000u64));
				admin_provider.send_transaction(tx).await?.get_receipt().await?;

				let balance = admin_provider.get_balance(address).await?;
				info!("setting up AWS Account:{address} granted Attester role of MCR contract with balance: {balance}");
			}
			SignerIdentifier::HashiCorpVault(_) => (),
		}

		config.settle.mcr_contract_address = mcr_address;

		Ok(config)
	}
}
