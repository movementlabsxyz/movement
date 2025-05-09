use crate::common_args::MovementArgs;
use anyhow::Context;
use aptos_sdk::types::account_address::AccountAddress;
use aptos_sdk::types::transaction::TransactionPayload;
use aptos_sdk::{
	move_types::{identifier::Identifier, language_storage::ModuleId},
	transaction_builder::TransactionBuilder,
	types::{transaction::EntryFunction, LocalAccount},
};
use clap::Parser;
use movement_da_light_node_client::MovementDaLightNodeClient;
use movement_da_light_node_proto::BatchWriteRequest;
use movement_da_light_node_proto::BlobWrite;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;
use tracing::info;

#[derive(Debug, Parser, Clone)]
#[clap(rename_all = "kebab-case", about = "Streams the DA blocks")]
pub struct SubmitBatch {
	#[clap(flatten)]
	pub movement_args: MovementArgs,
	pub light_node_url: String,
}

impl SubmitBatch {
	pub async fn execute(&self) -> Result<(), anyhow::Error> {
		// Get the config
		let dot_movement = self.movement_args.dot_movement()?;
		let config = dot_movement.try_get_config_from_json::<movement_config::Config>()?;

		let mut da_client = MovementDaLightNodeClient::try_http2(self.light_node_url.as_str())
			.await
			.context("Failed to connect to light node")?;

		let alice = LocalAccount::generate(&mut rand::rngs::OsRng);
		let bob = LocalAccount::generate(&mut rand::rngs::OsRng);
		// Create a raw transaction from Alice to Bob.
		let transaction_builder = TransactionBuilder::new(
			TransactionPayload::EntryFunction(EntryFunction::new(
				ModuleId::new(AccountAddress::from_str_strict("0x1")?, Identifier::new("coin")?),
				Identifier::new("transfer")?,
				vec![],
				vec![],
			)),
			SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() + 20,
			config.execution_config.maptos_config.chain.maptos_chain_id.clone(),
		)
		.sender(alice.address())
		.sequence_number(alice.sequence_number())
		.max_gas_amount(5_000)
		.gas_unit_price(100);

		// Sign the Tx by bob to be invalid. Just test the submit_batch don't want to be executed.
		let signed_transaction = bob.sign_with_transaction_builder(transaction_builder);
		let mut transactions = vec![];
		let serialized_aptos_transaction = bcs::to_bytes(&signed_transaction)?;
		let movement_transaction = movement_types::transaction::Transaction::new(
			serialized_aptos_transaction,
			0,
			signed_transaction.sequence_number(),
		);
		let serialized_transaction = serde_json::to_vec(&movement_transaction)?;
		transactions.push(BlobWrite { data: serialized_transaction });
		let batch_write = BatchWriteRequest { blobs: transactions };

		// write the batch to the DA
		let batch_write_reponse = da_client.batch_write(batch_write).await?;

		info!("Batch submitted with response: {batch_write_reponse:?}");

		Ok(())
	}
}
