// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0
use anyhow::Context;
use aptos_config::keys::ConfigKey;
use aptos_faucet_core::{
	funder::{
			mint::MintFunder, transfer::{
			AmountToFund, MinimumFunds, TransferFunderConfig
		}, ApiConnectionConfig, FunderConfig, FunderTrait, TransactionSubmissionConfig
	},
	server::{FunderKeyEnum, RunConfig}
};
use tracing::info;
use aptos_sdk::{
	// transaction_builder::TransactionFactory,
	// types::transaction::{TransactionArgument, Script},
	types::{
		account_config::aptos_test_root_address,
		LocalAccount
	},
	rest_client::Client,
};
// use aptos_faucet_core::funder::mint::MINTER_SCRIPT;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {

	aptos_logger::Logger::builder()
	.level(aptos_logger::Level::Info)
	.build();

	// sleep for a minute
	tokio::time::sleep(std::time::Duration::from_secs(10)).await;

	/*use tracing_subscriber::EnvFilter;

	tracing_subscriber::fmt()
		.with_env_filter(
			EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
		)
		.init();*/

	let dot_movement = dot_movement::DotMovement::try_from_env()?;
	let config = dot_movement.try_get_config_from_json::<suzuka_config::Config>()?;

	// get the connection url
	let connection_host =
		config.execution_config.maptos_config.faucet.maptos_rest_connection_hostname;
	let connection_port = config.execution_config.maptos_config.faucet.maptos_rest_connection_port;
	let connection_url = format!("http://{}:{}", connection_host, connection_port);

	// get the key
	let private_key = config.execution_config.maptos_config.chain.maptos_private_key.clone();

	// get the chain id
	let chain_id = config.execution_config.maptos_config.chain.maptos_chain_id.clone();

	let rest_client = Client::new(connection_url.parse()?);
	let sequence_number = rest_client.get_account(aptos_test_root_address()).await?.into_inner().sequence_number;

	info!("Minting delegated account using address {} and sequence number {}", aptos_test_root_address(), sequence_number);

	let api_connection_config = ApiConnectionConfig::new(
		connection_url.parse()?,
		// The config will use an encoded key if one is provided
		"/not/a/real/path".to_string().into(),
		Some(ConfigKey::new(private_key)),
		chain_id,
	);

	let transaction_submission_config = TransactionSubmissionConfig::new(
		None,
		None,
		60, 
		None, 
		500_000,
		60,
		360,
		true
	);


	let faucet_account = LocalAccount::new(
		aptos_test_root_address(),
		api_connection_config.get_key().context("Failed to get key")?,
		sequence_number,
	);

	/*let transaction_factory = TransactionFactory::new(chain_id);
	let delegated_account = LocalAccount::generate(&mut rand::rngs::OsRng);
	let signed_transaction = faucet_account.sign_with_transaction_builder(transaction_factory.script(
		Script::new(MINTER_SCRIPT.to_vec(), vec![], vec![
			TransactionArgument::Address(delegated_account.address()),
			TransactionArgument::U64(100_000_000_000_000),
		]),
	));

	rest_client.submit_and_wait_bcs(&signed_transaction).await.context(
		"failed to run self mint transaction",
	)?;*/

	let mut mint_funder = MintFunder::new(
		connection_url.parse()?,
		chain_id,
		transaction_submission_config.clone(),
		faucet_account
	);
	mint_funder.use_delegated_account().await?;

	let delegated_account = LocalAccount::generate(&mut rand::rngs::OsRng);
	info!("Minting delegated account with address: {}", delegated_account.address());
	let transactions = mint_funder.fund(
		Some(100_000_000_000_000),
		delegated_account.address(),
		false,
		false,
	).await.context(
		"Failed to mint delegated account"
	)?;
	for transaction in transactions {
		info!("Waiting for confirmation of transaction: {:?}", transaction);
		rest_client.wait_for_signed_transaction(&transaction).await?;
	}

	info!("Setting up transfer faucet run config");
	// get the listener host and port
	let listener_host =
		config.execution_config.maptos_config.faucet.maptos_faucet_rest_listen_hostname;
	let listener_port = config.execution_config.maptos_config.faucet.maptos_faucet_rest_listen_port;
	let mut config = RunConfig::build_for_cli(
		connection_url.parse()?, 
		listener_host, 
		listener_port, 
		FunderKeyEnum::Key(ConfigKey::new(delegated_account.private_key().clone())), 
		false, 
		Some(chain_id)
	);

	let transfer_config = TransferFunderConfig {
		api_connection_config,
		transaction_submission_config,
		minimum_funds : MinimumFunds(10_000_000_000),
		amount_to_fund : AmountToFund(2000),
	};

	config.funder_config = FunderConfig::TransferFunder(transfer_config);

	info!("Running faucet with config: {:?}", config);

	config.run().await?;

	Ok(())

	
}