use aptos_framework_core_resources_rotation_test::cached::PreL1Merge;

use aptos_sdk::crypto::SigningKey;
use aptos_sdk::{
	coin_client::CoinClient,
	rest_client::{Client, FaucetClient, Transaction},
	transaction_builder::TransactionFactory,
	types::{account_address::AccountAddress, transaction::TransactionPayload},
};
use aptos_types::{
	account_config::{RotationProofChallenge, CORE_CODE_ADDRESS},
	chain_id::ChainId,
	transaction::EntryFunction,
};
use maptos_framework_release_util::{LocalAccountReleaseSigner, Release};
use movement_client::types::{account_config::aptos_test_root_address, LocalAccount};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use tracing::info;
use url::Url;

const GAS_UNIT_LIMIT: u64 = 100_000;

static MOVEMENT_CONFIG: Lazy<movement_config::Config> = Lazy::new(|| {
	let dot_movement = dot_movement::DotMovement::try_from_env().unwrap();
	dot_movement.try_get_config_from_json::<movement_config::Config>().unwrap()
});

static NODE_URL: Lazy<Url> = Lazy::new(|| {
	let addr = MOVEMENT_CONFIG
		.execution_config
		.maptos_config
		.client
		.maptos_rest_connection_hostname
		.clone();
	let port = MOVEMENT_CONFIG
		.execution_config
		.maptos_config
		.client
		.maptos_rest_connection_port
		.clone();
	Url::from_str(&format!("http://{}:{}", addr, port)).unwrap()
});

static FAUCET_URL: Lazy<Url> = Lazy::new(|| {
	let addr = MOVEMENT_CONFIG
		.execution_config
		.maptos_config
		.client
		.maptos_faucet_rest_connection_hostname
		.clone();
	let port = MOVEMENT_CONFIG
		.execution_config
		.maptos_config
		.client
		.maptos_faucet_rest_connection_port;
	Url::from_str(&format!("http://{}:{}", addr, port)).unwrap()
});

#[derive(Serialize, Deserialize)]
struct RotationCapabilityOfferProofChallengeV2 {
	account_address: AccountAddress,
	module_name: String,
	struct_name: String,
	chain_id: u8,
	sequence_number: u64,
	source_address: AccountAddress,
	recipient_address: AccountAddress,
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
	use aptos_crypto::{ed25519::Ed25519PublicKey, ValidCryptoMaterial};
	use aptos_types::account_address::AccountAddress;
	use aptos_types::transaction::authenticator::AuthenticationKey;
	use tracing::info;

	tracing_subscriber::fmt().with_env_filter("info").init();

	let rest_client = Client::new(NODE_URL.clone());
	let faucet_client = FaucetClient::new(FAUCET_URL.clone(), NODE_URL.clone());

	// Governance release object
	let pre_l1_merge = PreL1Merge::new();

	// Core resources (aptos_test_root) address
	let gov_root_address = aptos_test_root_address();
	info!("aptos_test_root_address() (constant): {}", gov_root_address);

	// Load *core_resources* private key (from your config/genesis)
	let raw_private_key = MOVEMENT_CONFIG
		.execution_config
		.maptos_config
		.chain
		.maptos_private_key_signer_identifier
		.try_raw_private_key()?;
	let gov_priv =
		movement_client::crypto::ed25519::Ed25519PrivateKey::try_from(raw_private_key.as_slice())?;

	// Build signer by *forcing* core_resources address + current on-chain seq
	let mut gov_root_account = {
		let onchain = rest_client.get_account(gov_root_address).await?.into_inner();
		LocalAccount::new(gov_root_address, gov_priv.clone(), onchain.sequence_number)
	};
	info!("Signer (gov_root_account) address: {}", gov_root_account.address());

	// Fund signer and recipient (localnet)
	faucet_client.fund(gov_root_account.address(), 100_000_000_000).await?;
	let recipient = LocalAccount::generate(&mut rand::rngs::OsRng);
	faucet_client.fund(recipient.address(), 100_000_000_000).await?;

	// Helper to refresh the sequence JUST-IN-TIME before each tx
	async fn refresh_seq(client: &Client, acct: &mut LocalAccount) -> anyhow::Result<u64> {
		let info = client.get_account(acct.address()).await?.into_inner();
		// LocalAccount in movement_client exposes a constructor; safest is to rebuild:
		let pk = acct.private_key().clone();
		*acct = LocalAccount::new(acct.address(), pk, info.sequence_number);
		Ok(info.sequence_number)
	}

	// --- Offer Rotation Capability ---
	let ledger_info = rest_client.get_ledger_information().await?.into_inner();

	// Use fresh seq for the *challenge* (do not pre-increment)
	let seq_for_offer = refresh_seq(&rest_client, &mut gov_root_account).await?;
	let rotation_capability_proof = RotationCapabilityOfferProofChallengeV2 {
		account_address: CORE_CODE_ADDRESS,
		module_name: "account".to_string(),
		struct_name: "RotationCapabilityOfferProofChallengeV2".to_string(),
		chain_id: ledger_info.chain_id,
		sequence_number: seq_for_offer,
		source_address: gov_root_account.address(),
		recipient_address: recipient.address(),
	};

	let proof_msg = bcs::to_bytes(&rotation_capability_proof)?;
	let proof_signed = gov_root_account.private_key().sign_arbitrary_message(&proof_msg);

	let offer_payload = make_entry_function_payload(
		CORE_CODE_ADDRESS,
		"account",
		"offer_rotation_capability",
		vec![],
		vec![
			bcs::to_bytes(&proof_signed.to_bytes().to_vec())?,
			bcs::to_bytes(&0u8)?, // from_scheme = Ed25519
			bcs::to_bytes(&gov_root_account.public_key().to_bytes().to_vec())?,
			bcs::to_bytes(&recipient.address())?,
		],
	)?;

	// Refresh seq again right before submit (in case another tx raced)
	refresh_seq(&rest_client, &mut gov_root_account).await?;
	send_aptos_transaction(&rest_client, &mut gov_root_account, offer_payload).await?;
	info!(" Offer rotation capability submitted.");

	// --- Rotate Authentication Key ---
	let seq_for_rotate = refresh_seq(&rest_client, &mut gov_root_account).await?;
	let rotation_proof = RotationProofChallenge {
		account_address: CORE_CODE_ADDRESS,
		module_name: "account".to_string(),
		struct_name: "RotationProofChallenge".to_string(),
		sequence_number: seq_for_rotate,
		originator: gov_root_account.address(),
		current_auth_key: AccountAddress::from_bytes(gov_root_account.authentication_key())?,
		new_public_key: recipient.public_key().to_bytes().to_vec(),
	};

	let rotation_msg = bcs::to_bytes(&rotation_proof)?;
	let sig_curr = gov_root_account.private_key().sign_arbitrary_message(&rotation_msg);
	let sig_new = recipient.private_key().sign_arbitrary_message(&rotation_msg);

	let rotate_payload = make_entry_function_payload(
		CORE_CODE_ADDRESS,
		"account",
		"rotate_authentication_key",
		vec![],
		vec![
			bcs::to_bytes(&0u8)?, // from_scheme = Ed25519
			bcs::to_bytes(&gov_root_account.public_key().to_bytes().to_vec())?,
			bcs::to_bytes(&0u8)?, // to_scheme = Ed25519
			bcs::to_bytes(&recipient.public_key().to_bytes().to_vec())?,
			bcs::to_bytes(&sig_curr.to_bytes().to_vec())?,
			bcs::to_bytes(&sig_new.to_bytes().to_vec())?,
		],
	)?;

	// Refresh seq again just before submit
	refresh_seq(&rest_client, &mut gov_root_account).await?;
	send_aptos_transaction(&rest_client, &mut gov_root_account, rotate_payload).await?;
	info!("✅ Authentication key rotated successfully.");

	// --- Verify Rotation (read back the *same* account you rotated) ---
	let updated_info = rest_client.get_account(gov_root_account.address()).await?.into_inner();

	// Compute expected auth key exactly as the framework
	let recip_pub = Ed25519PublicKey::try_from(recipient.public_key().to_bytes().as_slice())
		.expect("recipient pubkey parse");
	let expected_auth_key = AuthenticationKey::ed25519(&recip_pub);

	info!("on-chain auth_key:   {:?}", updated_info.authentication_key);
	info!("expected auth_key:   {:?}", expected_auth_key);
	info!("helper  auth_key(?): {:?}", recipient.authentication_key());

	assert_eq!(
		updated_info.authentication_key, expected_auth_key,
		"On-chain authentication key must match expected ed25519 recipient key"
	);

	// --- Build rotated governance signer for subsequent governance actions ---
	let rotated_gov_account = LocalAccount::new(
		gov_root_account.address(),
		recipient.private_key().clone(),
		updated_info.sequence_number,
	);
	let rotated_release_signer =
		LocalAccountReleaseSigner::new(rotated_gov_account, Some(aptos_test_root_address()));

	// --- Submit governance proposal with rotated signer ---
	let move_rest_client = movement_client::rest_client::Client::new(NODE_URL.clone());
	pre_l1_merge
		.release(&rotated_release_signer, 2_000_000, 100, 60, &move_rest_client)
		.await?;

	info!("✅ Governance release successfully signed using rotated aptos_test_root_address!");
	Ok(())
}

// --- Helpers ---
fn make_entry_function_payload(
	package_address: AccountAddress,
	module_name: &'static str,
	function_name: &'static str,
	ty_args: Vec<aptos_sdk::move_types::language_storage::TypeTag>,
	args: Vec<Vec<u8>>,
) -> Result<TransactionPayload, anyhow::Error> {
	let module_id = aptos_sdk::move_types::language_storage::ModuleId::new(
		package_address,
		aptos_sdk::move_types::identifier::Identifier::new(module_name)?,
	);
	let function_id = aptos_sdk::move_types::identifier::Identifier::new(function_name)?;
	Ok(TransactionPayload::EntryFunction(EntryFunction::new(module_id, function_id, ty_args, args)))
}

async fn send_aptos_transaction(
	client: &Client,
	signer: &mut LocalAccount,
	payload: TransactionPayload,
) -> anyhow::Result<Transaction> {
	let state = client.get_ledger_information().await?.into_inner();
	let factory = TransactionFactory::new(ChainId::new(state.chain_id))
		.with_gas_unit_price(100)
		.with_max_gas_amount(GAS_UNIT_LIMIT);
	let signed_tx = signer.sign_with_transaction_builder(factory.payload(payload));
	Ok(client.submit_and_wait(&signed_tx).await?.into_inner())
}
