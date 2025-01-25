use anyhow::Context;
use aptos_sdk::crypto::ed25519::Ed25519PrivateKey;
use aptos_sdk::crypto::hash::{CryptoHash, DefaultHasher};
use aptos_sdk::crypto::{HashValue, SigningKey, Uniform, ValidCryptoMaterialStringExt};
use aptos_sdk::{
	crypto::test_utils::KeyPair,
	rest_client::{Client, FaucetClient},
	types::account_address::AccountAddress,
	types::transaction::{Script, TransactionArgument, TransactionPayload},
};
use aptos_types::account_config::RotationProofChallenge;
use movement_client::crypto::hash::CryptoHasher;
use movement_client::{coin_client::CoinClient, crypto::ed25519::PublicKey, types::LocalAccount};
use once_cell::sync::{Lazy, OnceCell};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use std::{fs, str::FromStr};
use url::Url;

// This has come tiredly from https://github.com/movementlabsxyz/aptos-core/blob/ac9de113a4afec6a26fe587bb92c982532f09d3a/crates/aptos-crypto/src/hash.rs#L556-L557
// I was unbale to import it and its not feature flagged, copying here for speed sake.
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

static SUZUKA_CONFIG: Lazy<movement_config::Config> = Lazy::new(|| {
	let dot_movement = dot_movement::DotMovement::try_from_env().unwrap();
	let config = dot_movement.try_get_config_from_json::<movement_config::Config>().unwrap();
	config
});

static NODE_URL: Lazy<Url> = Lazy::new(|| {
	let node_connection_address = SUZUKA_CONFIG
		.execution_config
		.maptos_config
		.client
		.maptos_rest_connection_hostname
		.clone();
	let node_connection_port = SUZUKA_CONFIG
		.execution_config
		.maptos_config
		.client
		.maptos_rest_connection_port
		.clone();
	let node_connection_url =
		format!("http://{}:{}", node_connection_address, node_connection_port);
	Url::from_str(&node_connection_url).unwrap()
});

static FAUCET_URL: Lazy<Url> = Lazy::new(|| {
	let faucet_listen_address = SUZUKA_CONFIG
		.execution_config
		.maptos_config
		.client
		.maptos_faucet_rest_connection_hostname
		.clone();
	let faucet_listen_port = SUZUKA_CONFIG
		.execution_config
		.maptos_config
		.client
		.maptos_faucet_rest_connection_port
		.clone();
	let faucet_listen_url = format!("http://{}:{}", faucet_listen_address, faucet_listen_port);
	Url::from_str(&faucet_listen_url).unwrap()
});

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RotationMessage(Vec<u8>);

define_hasher!(RotationMessageHasher, ROTATION_MESSAGE_HASHER, b"RotationMessage");

define_hasher! {
	(
		RotationMessageHasher,
		TRANSACTION_ACCUMULATOR_HASHER,
		TRANSACTION_ACCUMULATOR_SEED,
		b"RotationMessage"
	)
}

impl CryptoHash for RotationMessage {
	type Hasher = RotationMessageHasher;

	fn hash(&self) -> HashValue {
		let mut hasher = Self::Hasher::default();
		hasher.update(&self.0);
		hasher.finish()
	}
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
	// Initialize clients
	let rest_client = Client::new(NODE_URL.clone());
	let faucet_client = FaucetClient::new(FAUCET_URL.clone(), NODE_URL.clone());
	let coin_client = CoinClient::new(&rest_client);

	// Get chain ID
	let chain_id = rest_client
		.get_index()
		.await
		.context("Failed to get chain ID")?
		.inner()
		.chain_id;

	// Load core resource account
	let mut core_resources_account = LocalAccount::from_private_key(
		SUZUKA_CONFIG
			.execution_config
			.maptos_config
			.chain
			.maptos_private_key
			.to_encoded_string()?
			.as_str(),
		0,
	)?;
	println!("Core Resources Account Address: {}", core_resources_account.address());

	tracing::info!("Core resources account loaded");

	// Generate sender and delegate accounts
	let mut sender = LocalAccount::generate(&mut rand::rngs::OsRng);
	let delegate = LocalAccount::generate(&mut rand::rngs::OsRng);

	tracing::info!("Generated sender and delegate accounts");

	// Fund the sender account
	faucet_client
		.fund(sender.address(), 1_000_000)
		.await
		.context("Failed to fund sender account")?;

	// Generate new key pair for rotation using KeyPair
	let new_keypair: KeyPair<Ed25519PrivateKey, PublicKey> =
		KeyPair::generate(&mut rand::rngs::OsRng);
	let new_public_key: PublicKey = new_keypair.public_key.clone();

	// Create the rotation proof challenge
	let rotation_proof = RotationProofChallenge {
		module_name: String::from("account"),
		struct_name: String::from("RotationProofChallenge"),
		account_address: sender.address(),
		sequence_number: sender.sequence_number(),
		originator: sender.address(),
		current_auth_key: AccountAddress::from_str(
			core_resources_account.private_key().to_encoded_string().unwrap().as_str(),
		)?,
		new_public_key: Vec::from(new_public_key.to_bytes()),
	};

	let rotation_message = bcs::to_bytes(&rotation_proof).unwrap();

	// Sign the rotation message directly using the private key
	let signature_by_new_privkey = new_keypair.private_key.sign(&rotation_message);

	// Read the compiled Move script
	let script_code = fs::read("path/to/compiled/script.mv").context("Failed to read script")?;
	let script_payload = TransactionPayload::Script(Script::new(
		script_code,
		vec![],
		vec![
			TransactionArgument::U8(0), // Scheme for the current key (Ed25519)
			TransactionArgument::U8(0), // Scheme for the new key (Ed25519)
			TransactionArgument::Bytes(new_public_key.to_bytes().to_vec()),
			TransactionArgument::Bytes(signature_by_new_privkey.to_bytes().to_vec()),
		],
	));

	// Create and submit the transaction
	let expiration_time = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() + 60; // 60 seconds from now
	let txn = transaction_test_helpers::get_test_signed_transaction_with_chain_id(
		sender.address(),
		sender.sequence_number(),
		sender.private_key(),
		sender.public_key(),
		Some(script_payload),
		expiration_time,
		100, // Max gas
		None,
		chain_id,
	);

	tracing::info!("Submitting transaction for key rotation");
	rest_client
		.submit_and_wait(&txn)
		.await
		.context("Failed to submit key rotation transaction")?;

	tracing::info!("Key rotation transaction completed successfully");

	Ok(())
}
