use anyhow::Context;
use aptos_sdk::crypto::ed25519::Ed25519PrivateKey;
use aptos_sdk::crypto::ed25519::Ed25519PublicKey;
use aptos_sdk::crypto::ed25519::PrivateKey as EdPrivateKey;
use aptos_sdk::crypto::PrivateKey;
use aptos_sdk::move_types::identifier::Identifier;
use aptos_sdk::move_types::language_storage::ModuleId;
use aptos_sdk::move_types::language_storage::TypeTag;
use aptos_sdk::transaction_builder::TransactionBuilder;
use aptos_sdk::types::account_address::AccountAddress;
use aptos_sdk::types::chain_id::ChainId;
use aptos_sdk::types::transaction::authenticator::AuthenticationKey;
use aptos_sdk::types::transaction::EntryFunction;
use aptos_sdk::types::transaction::SignedTransaction;
use aptos_sdk::types::transaction::TransactionPayload;
use aptos_sdk::types::LocalAccount;
use bcs::to_bytes;
use once_cell::sync::Lazy;
use serde_json::{json, Value};
use std::fs;
use std::path::Path;
use std::str::FromStr;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;
use url::Url;

pub static ACCOUNT_LIST: Lazy<Vec<(EdPrivateKey, AccountAddress)>> = Lazy::new(|| {
	if let Some(account_file) = std::env::var_os("TEST_ALICE_BOB_ACCOUNT_FILE_PATH") {
		let account_file = Path::new(&account_file);
		if account_file.exists() {
			match load_accounts_from_file(account_file) {
				Ok(account_list) => account_list,
				Err(err) => {
					tracing::error!("Can't load account file: {err}.");
					vec![]
				}
			}
		} else {
			tracing::error!("File specified in TEST_ALICE_BOB_ACCOUNT_FILE_PATH env var doesn't exist :{account_file:?}");
			vec![]
		}
	} else {
		vec![]
	}
});

pub static SUZUKA_CONFIG: Lazy<movement_config::Config> = Lazy::new(|| {
	let dot_movement = dot_movement::DotMovement::try_from_env().unwrap();
	let config = dot_movement.try_get_config_from_json::<movement_config::Config>().unwrap();
	config
});

pub static NODE_URL: Lazy<Url> = Lazy::new(|| {
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

	let node_connection_url = std::env::var_os("TEST_ALICE_BOB_RPC_URL")
		.map(|str| str.to_string_lossy().into_owned())
		.unwrap_or_else(|| format!("http://{}:{}", node_connection_address, node_connection_port));
	Url::from_str(node_connection_url.as_str()).unwrap()
});

pub static FAUCET_URL: Lazy<Url> = Lazy::new(|| {
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
	let faucet_listen_url = std::env::var_os("TEST_ALICE_BOB_FAUCET_URL")
		.map(|str| str.to_string_lossy().into_owned())
		.unwrap_or_else(|| format!("http://{}:{}", faucet_listen_address, faucet_listen_port));

	Url::from_str(faucet_listen_url.as_str()).unwrap()
});

pub fn get_account_from_list(
	index: usize,
) -> Result<(EdPrivateKey, AccountAddress), anyhow::Error> {
	let accounts: &Vec<(EdPrivateKey, AccountAddress)> = ACCOUNT_LIST.as_ref();
	accounts.get(index).cloned().with_context(|| {
		format!(
			"Not enough account in accounts file ({}) for the number of scenarios {} .",
			accounts.len(),
			index,
		)
	})
}

pub fn load_accounts_from_file<P: AsRef<Path>>(
	file_path: P,
) -> Result<Vec<(Ed25519PrivateKey, AccountAddress)>, anyhow::Error> {
	let data = fs::read_to_string(&file_path)?;

	// Parse the JSON into a generic `Value`
	let v: Value = serde_json::from_str(&data)?;

	// Expect an array at the top level
	let arr = v.as_array().context("Json account file: Expected top-level JSON array")?;

	arr.into_iter()
		.map(|obj| {
			// Each element should be an object
			let map = obj
				.as_object()
				.context("Json account file: Expected each array element to be an object")?;

			// Extract the fields as strings
			let _public_key = map
				.get("public_key")
				.and_then(Value::as_str)
				.context("Json account file: public_key missing or not a string")?;
			let private_key = map
				.get("private_key")
				.and_then(Value::as_str)
				.context("Json account file: private_key missing or not a string")?
				.trim_start_matches("0x"); //.trim_start_matches("0x")

			let private_key = Ed25519PrivateKey::try_from(hex::decode(private_key)?.as_slice())?;
			let pubkey = private_key.public_key();
			let auth_key = AuthenticationKey::ed25519(&pubkey);
			let account_address: AccountAddress = auth_key.account_address();

			Ok((private_key, account_address))
		})
		.collect::<Result<Vec<_>, _>>()
}

pub fn add_account_to_accounts_file<P: AsRef<Path>>(
	path: P,
	public_key: &Ed25519PublicKey,
	private_key: &Ed25519PrivateKey,
) -> Result<(), anyhow::Error> {
	let data = fs::read_to_string(&path)?;
	let mut v: Value = serde_json::from_str(&data)?;
	let arr = v.as_array_mut().ok_or(anyhow::anyhow!("expected top-level JSON array"))?;
	let new_entry = json!({
		"public_key":  format!("0x{}", hex::encode(public_key.to_bytes())),
		"private_key": format!("0x{}", hex::encode(private_key.to_bytes())),
	});
	arr.push(new_entry);
	let pretty = serde_json::to_string_pretty(&v)?;
	fs::write(&path, pretty)?;

	Ok(())
}

pub async fn create_signed_transfer_transaction(
	chain_id: u8,
	from_account: &LocalAccount,
	to_account: AccountAddress,
	amount: u64,
	sequence_number: u64,
) -> Result<SignedTransaction, anyhow::Error> {
	let coin_type = "0x1::aptos_coin::AptosCoin";
	let max_gas_amount = 5_000;
	let gas_unit_price = 100;

	let transaction_builder = TransactionBuilder::new(
		TransactionPayload::EntryFunction(EntryFunction::new(
			ModuleId::new(AccountAddress::ONE, Identifier::new("coin").unwrap()),
			Identifier::new("transfer")?,
			vec![TypeTag::from_str(coin_type)?],
			vec![to_bytes(&to_account)?, to_bytes(&amount)?],
		)),
		SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() + 60,
		ChainId::new(chain_id),
	);

	let raw_transaction = transaction_builder
		.sender(from_account.address())
		.sequence_number(sequence_number)
		.max_gas_amount(max_gas_amount)
		.gas_unit_price(gas_unit_price)
		.chain_id(ChainId::new(chain_id))
		.build();

	let signed_transaction = from_account.sign_transaction(raw_transaction);

	Ok(signed_transaction)
}
