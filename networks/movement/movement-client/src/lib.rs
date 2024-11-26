pub mod load_soak_testing;
#[cfg(test)]
pub mod tests;

pub use aptos_sdk::*;
pub use movement_da_light_node_client::*;
pub use movement_da_light_node_proto::*;
pub use movement_types;

#[cfg(test)]
pub mod test {

	use tracing::info;

	use super::{
		move_types::identifier::Identifier,
		move_types::language_storage::ModuleId,
		transaction_builder::TransactionBuilder,
		types::account_address::AccountAddress,
		types::chain_id::ChainId,
		types::transaction::{EntryFunction, TransactionPayload},
		types::LocalAccount,
	};
	use std::time::{SystemTime, UNIX_EPOCH};

	#[tokio::test]
	#[tracing_test::traced_test]
	pub async fn test_transaction_size() -> Result<(), anyhow::Error> {
		let mut alice = LocalAccount::generate(&mut rand::rngs::OsRng);
		let transaction_builder = TransactionBuilder::new(
			TransactionPayload::EntryFunction(EntryFunction::new(
				ModuleId::new(AccountAddress::from_str_strict("0x1")?, Identifier::new("coin")?),
				Identifier::new("transfer")?,
				vec![],
				vec![],
			)),
			SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() + 20,
			ChainId::new(1),
		)
		.sender(alice.address())
		.sequence_number(alice.sequence_number())
		.max_gas_amount(5_000)
		.gas_unit_price(100);

		// create the blob write
		let signed_transaction = alice.sign_with_transaction_builder(transaction_builder);
		let txn_hash = signed_transaction.committed_hash();
		let serialized_aptos_transaction = bcs::to_bytes(&signed_transaction)?;
		info!("transaction size {}", serialized_aptos_transaction.len());

		Ok(())
	}
}
