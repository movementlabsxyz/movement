use crate::batch::DaBatch;
use crate::tests::whitelist::make_test_whitelist;
use aptos_crypto::hash::CryptoHash;
use aptos_crypto::SigningKey;
use aptos_crypto::Uniform;
use aptos_sdk::crypto::ed25519::Ed25519PrivateKey;
use aptos_sdk::crypto::ed25519::Ed25519PublicKey;
use aptos_sdk::types::account_address::AccountAddress;
use aptos_sdk::types::chain_id::ChainId;
use aptos_sdk::types::transaction::RawTransaction;
use aptos_sdk::types::transaction::Script;
use aptos_sdk::types::transaction::SignedTransaction;
use aptos_sdk::types::transaction::TransactionPayload;
use ed25519_dalek::Signer;
use ed25519_dalek::SigningKey as DalekSigningKey;
use rand::rngs::OsRng;
use rand::RngCore;
use serde::Serialize;

pub mod client;
pub mod mock;
pub mod whitelist;

impl<D> DaBatch<D>
where
	D: Serialize + CryptoHash,
{
	/// Creates a test-only `DaBatch` with a real signature over the given data.
	/// Only usable in tests.
	pub fn test_only_new(data: D) -> Self
	where
		D: Serialize,
	{
		let private_key = generate_signing_key();
		let public_key = private_key.verifying_key();

		let serialized = bcs::to_bytes(&data).unwrap(); // only fails if serialization is broken
		let signature = private_key.sign(&serialized);

		Self { data, signature, signer: public_key }
	}
}

pub fn generate_signing_key() -> DalekSigningKey {
	let mut bytes = [0u8; 32];
	OsRng.fill_bytes(&mut bytes);
	let signing_key = DalekSigningKey::from_bytes(&bytes);
	signing_key
}

pub fn create_aptos_transaction() -> SignedTransaction {
	let transaction_payload = TransactionPayload::Script(Script::new(vec![0], vec![], vec![]));
	let raw_transaction = RawTransaction::new(
		AccountAddress::random(),
		0,
		transaction_payload,
		0,
		0,
		0,
		ChainId::test(), // This is the value used in aptos testing code.
	);

	let private_key = Ed25519PrivateKey::generate_for_testing();
	let public_key = Ed25519PublicKey::from(&private_key);

	let signature = private_key.sign(&raw_transaction).unwrap();
	SignedTransaction::new(raw_transaction, public_key, signature)
}
