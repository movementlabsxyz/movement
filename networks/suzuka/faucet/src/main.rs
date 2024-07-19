// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0
use anyhow::{Context, Result};
use aptos_config::keys::ConfigKey;
use aptos_faucet_core::{
	funder::{
		ApiConnectionConfig,
		FunderConfig,
		mint::MINTER_SCRIPT,
		transfer::{
			TransferFunderConfig,
			MinimumFunds,
			AmountToFund,
		},
		TransactionSubmissionConfig
	},
	server::{Server, RunConfig, FunderKeyEnum}
};
use tracing::info;
use aptos_sdk::{
	crypto::{
		ed25519::Ed25519PrivateKey, PrivateKey, ValidCryptoMaterialStringExt
	}, rest_client::Client, transaction_builder::{TransactionFactory, aptos_stdlib}, types::{
		account_address::AccountAddress, chain_id::ChainId, transaction::{
			authenticator::AuthenticationKey, Script, TransactionArgument
		}, LocalAccount
	},
};
use aptos_rest_client::{
    aptos_api_types::{AptosError, AptosErrorCode},
    error::{AptosErrorResponse, RestError},
};
use clap::Parser;

#[derive(Clone, Debug, Parser)]
pub struct Args {
	#[clap(subcommand)]
	server: Server,
}

impl Args {
	pub async fn run_command(&self) -> Result<()> {
		self.server.run_command().await
	}
}

pub async fn self_mint(
	private_key : &Ed25519PrivateKey,
	rest_client : &Client,
	chain_id : ChainId,
	amount : u64,
) -> Result<LocalAccount, anyhow::Error> {

	let public_key = private_key.public_key();
    let auth_key = AuthenticationKey::ed25519(&public_key);
    let address = AccountAddress::new(*auth_key.account_address());
	let sequence_number = {
		match rest_client.get_account(address).await{
			Ok(account) => account.into_inner().sequence_number,
			Err(err) => {
                if let RestError::Api(AptosErrorResponse {
                    error:
                        AptosError {
                            error_code: AptosErrorCode::ResourceNotFound,
                            ..
                        },
                    ..
                })
                | RestError::Api(AptosErrorResponse {
                    error:
                        AptosError {
                            error_code: AptosErrorCode::AccountNotFound,
                            ..
                        },
                    ..
                }) = err
                {
					info!("Account not found, assuming sequence number 1");
                    1
                } else {
                    anyhow::bail!("failed to get account state: {}", err)
                }
			}
		}
	};

	info!("Self minting {} to address {} with sequence number {}", amount, address, sequence_number);
	let local_account = LocalAccount::new(
		address,
		private_key.clone(),
		sequence_number,
	);

	let transaction_factory = TransactionFactory::new(chain_id);
	rest_client
            .submit_and_wait(&local_account.sign_with_transaction_builder(
                transaction_factory.payload(aptos_stdlib::aptos_coin_claim_mint_capability()),
            ))
            .await
            .context("Failed to claim the minting capability")?;

	let signed_transaction = local_account.sign_with_transaction_builder(transaction_factory.script(
		Script::new(MINTER_SCRIPT.to_vec(), vec![], vec![
			TransactionArgument::Address(address),
			TransactionArgument::U64(amount),
		]),
	));

	rest_client.submit_and_wait_bcs(&signed_transaction).await.context(
		"failed to run self mint transaction",
	)?;

	Ok(local_account)

}

pub async fn self_mint_with_retries(
	private_key : &Ed25519PrivateKey,
	rest_client : &Client,
	chain_id : ChainId,
	amount : u64,
	retries : u64,
) -> Result<LocalAccount, anyhow::Error> {
	for _ in 0..retries {
		match self_mint(private_key, rest_client, chain_id, amount).await {
			Ok(local_account) => return Ok(local_account),
			Err(e) => {
				tracing::warn!("Failed to self mint: {}", e);
			}
		}
		tokio::time::sleep(std::time::Duration::from_secs(1)).await;
	}
	Err(anyhow::anyhow!("Failed to self mint after {} retries", retries))
}

#[tokio::main]
async fn main() -> Result<()> {

	use tracing_subscriber::EnvFilter;

	tracing_subscriber::fmt()
		.with_env_filter(
			EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
		)
		.init();

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

	// get the listener host and port
	let listener_host =
		config.execution_config.maptos_config.faucet.maptos_faucet_rest_listen_hostname;
	let listener_port = config.execution_config.maptos_config.faucet.maptos_faucet_rest_listen_port;

	// self mint for the faucet
	info!("Self minting for the faucet");
	let rest_client = Client::new(connection_url.clone().parse()?);
	let _local_client = self_mint_with_retries(&private_key, &rest_client, chain_id, 10_000_000_000, 20).await?;

	let mut config = RunConfig::build_for_cli(
		connection_url.parse()?, 
		listener_host, 
		listener_port, 
		FunderKeyEnum::Key(ConfigKey::new(private_key.clone())), 
		true, 
		Some(chain_id)
	);

	let transfer_config = TransferFunderConfig {
		api_connection_config : ApiConnectionConfig::new(
			connection_url.parse()?,
			"not/a/real/path".to_string().into(),
			Some(ConfigKey::new(private_key)),
			chain_id,
		),
		transaction_submission_config : TransactionSubmissionConfig::new(
			None,
			None,
			60, 
			None, 
			10_000_000,
			60,
			360,
			false
		),
		minimum_funds : MinimumFunds(10_000),
		amount_to_fund : AmountToFund(10_000_000),
	};

	config.funder_config = FunderConfig::TransferFunder(transfer_config);

	info!("Running faucet with config: {:?}", config);

	config.run().await?;

	Ok(())

	
}

#[test]
fn verify_tool() {
	use clap::CommandFactory;
	Args::command().debug_assert()
}
