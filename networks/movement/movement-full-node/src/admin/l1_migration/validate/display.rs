use crate::admin::l1_migration::validate::types::api::AptosRestClient;
use aptos_api_types::Transaction;
use aptos_crypto::HashValue;
use clap::Parser;

#[derive(Parser, Debug)]
#[clap(name = "da-height", about = "Extract synced block height from the DA-sequencer database")]
pub struct DisplayTransactionOutputs {
	#[clap(long = "api", help = "The url of an Aptos api endpoint")]
	pub api_url: String,
	#[arg(help = "Transaction hash")]
	hash: String,
}

impl DisplayTransactionOutputs {
	pub async fn run(&self) -> anyhow::Result<()> {
		let hash = HashValue::from_hex(self.hash.trim_start_matches("0x"))?;
		let aptos_rest_client = AptosRestClient::try_connect(&self.api_url).await?;
		display_txn_outputs(aptos_rest_client, hash).await?;
		Ok(())
	}
}

#[test]
fn verify_tool() {
	use clap::CommandFactory;
	DisplayTransactionOutputs::command().debug_assert()
}

async fn display_txn_outputs(
	aptos_rest_client: AptosRestClient,
	hash: HashValue,
) -> anyhow::Result<()> {
	let txn = aptos_rest_client.get_transaction_by_hash(hash).await?.into_inner();

	if let Transaction::UserTransaction(txn) = txn {
		println!("Events:\n{}", serde_json::to_string_pretty(&txn.events)?);
		println!("Write-Set:\n{}", serde_json::to_string_pretty(&txn.info.changes)?);
	} else {
		println!("Transaction is pending");
	}

	Ok(())
}
