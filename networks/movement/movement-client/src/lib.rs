pub mod load_soak_testing;
#[cfg(test)]
pub mod tests;

pub use aptos_sdk::*;
pub use movement_da_light_node_client::*;
pub use movement_da_light_node_proto::*;
pub use movement_types;

// This is taken directly from https://github.com/movementlabsxyz/aptos-core/blob/ac9de113a4afec6a26fe587bb92c982532f09d3a/crates/aptos-crypto/src/hash.rs#L556
// Was unable to import it, and its not feature flagged.
// For safe of speed I opted to copy the code here for now.
macro_rules! define_hasher {
    (
        $(#[$attr:meta])*
        ($hasher_type: ident, $hasher_name: ident, $seed_name: ident, $salt: expr)
    ) => {

        #[derive(Clone, Debug)]
        $(#[$attr])*
        pub struct $hasher_type(DefaultHasher);

        impl $hasher_type {
            fn new() -> Self {
                $hasher_type(DefaultHasher::new($salt))
            }
        }

        static $hasher_name: Lazy<$hasher_type> = Lazy::new(|| { $hasher_type::new() });
        static $seed_name: OnceCell<[u8; 32]> = OnceCell::new();

        impl Default for $hasher_type {
            fn default() -> Self {
                $hasher_name.clone()
            }
        }

        impl CryptoHasher for $hasher_type {
            fn seed() -> &'static [u8;32] {
                $seed_name.get_or_init(|| {
                    DefaultHasher::prefixed_hash($salt)
                })
            }

            fn update(&mut self, bytes: &[u8]) {
                self.0.update(bytes);
            }

            fn finish(self) -> HashValue {
                self.0.finish()
            }
        }

        impl std::io::Write for $hasher_type {
            fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
                self.0.update(bytes);
                Ok(bytes.len())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
    };
}

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
