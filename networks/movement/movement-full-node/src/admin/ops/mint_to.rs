#[allow(unused_imports)]
use anyhow::Context;
use aptos_sdk::types::{
	transaction::{Script, TransactionArgument, TransactionPayload},
	LocalAccount,
};
use aptos_types::{chain_id::ChainId, test_helpers::transaction_test_helpers};
use clap::Parser;
use movement_config::ops::aptos::rest_client::RestClientOperations;
use std::time::UNIX_EPOCH;
use std::{fs, time::SystemTime};
use tokio::process::Command;

use crate::common_args::MovementArgs;

#[derive(Debug, Parser, Clone)]
#[clap(
	rename_all = "kebab-case",
	about = "Mint token to a recipient with the core_resource account."
)]
pub struct MintTo {
	#[clap(flatten)]
	pub movement_args: MovementArgs,

	/// The amount to send
	#[clap(long, short)]
	amount: u64,

	/// The address of the recipient
	#[clap(long, short)]
	recipient: String,
}

impl MintTo {
	pub async fn execute(&self) -> Result<(), anyhow::Error> {
		let dot_movement = self.movement_args.dot_movement()?;
		let config = dot_movement.try_get_config_from_json::<movement_config::Config>()?;

		let rest_client = config.get_rest_client().await?;

		//let faucet_client = FaucetClient::new(FAUCET_URL.clone(), NODE_URL.clone());
		//let coin_client = CoinClient::new(&rest_client);

		let chain_id = rest_client
			.get_index()
			.await
			.context("failed to get chain ID")?
			.inner()
			.chain_id;

		let raw_private_key = config
			.execution_config
			.maptos_config
			.chain
			.maptos_private_key_signer_identifier
			.try_raw_private_key()?;

		let hex_string = hex::encode(raw_private_key.as_slice());
		//let private_key = Ed25519PrivateKey::from_encoded_string(&hex_string)?;

		let core_resources_account: LocalAccount = LocalAccount::from_private_key(&hex_string, 0)?;

		tracing::info!("Created core resources account");
		tracing::debug!("core_resources_account address: {}", core_resources_account.address());

		// I know that we shouldn't compile on cmd execution, but we can optimise this later.
		let _compile_status = Command::new("movement")
			.args([
				"move",
				"compile",
				"--package-dir",
				"protocol-units/bridge/move-modules/build/bridge-modules/bytecode_scripts/mint_to.mv"
			])
			.status()
			.await
			.expect("Failed to execute `movement compile` command");

		let mint_core_code = fs::read(
			"protocol-units/bridge/move-modules/build/bridge-modules/bytecode_scripts/mint_to.mv",
		)?;

		let mint_core_args = vec![
			TransactionArgument::Address(core_resources_account.address()),
			TransactionArgument::U64(self.amount),
		];
		let mint_core_script_payload =
			TransactionPayload::Script(Script::new(mint_core_code, vec![], mint_core_args));

		let mint_core_script_transaction =
			transaction_test_helpers::get_test_signed_transaction_with_chain_id(
				core_resources_account.address(),
				core_resources_account.sequence_number(),
				&core_resources_account.private_key(),
				core_resources_account.public_key().clone(),
				Some(mint_core_script_payload),
				SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() + 60,
				100,
				None,
				ChainId::new(chain_id),
			);

		rest_client
			.submit_and_wait(&mint_core_script_transaction)
			.await
			.context("Failed to execute mint core balance script transaction")?;

		Ok(())
	}
}
