use aptos_framework_pre_l1_merge_release::cached::full::feature_upgrade::PreL1Merge;
use maptos_framework_release_util::{LocalAccountReleaseSigner, Release};
use movement_client::types::{account_config::aptos_test_root_address, LocalAccount};
use once_cell::sync::Lazy;
use std::str::FromStr;
use url::Url;

static MOVEMENT_CONFIG: Lazy<movement_config::Config> = Lazy::new(|| {
	let dot_movement = dot_movement::DotMovement::try_from_env().unwrap();
	let config = dot_movement.try_get_config_from_json::<movement_config::Config>().unwrap();
	config
});

// :!:>section_1c
static NODE_URL: Lazy<Url> = Lazy::new(|| {
	let node_connection_address = MOVEMENT_CONFIG
		.execution_config
		.maptos_config
		.client
		.maptos_rest_connection_hostname
		.clone();
	let node_connection_port = MOVEMENT_CONFIG
		.execution_config
		.maptos_config
		.client
		.maptos_rest_connection_port
		.clone();

	let node_connection_url =
		format!("http://{}:{}", node_connection_address, node_connection_port);

	Url::from_str(node_connection_url.as_str()).unwrap()
});

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
	// setup the logger
	use tracing_subscriber::EnvFilter;

	tracing_subscriber::fmt()
		.with_env_filter(
			EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
		)
		.init();

	// form the elsa release
	let biarritz_rc1 = PreL1Merge::new();

	// get the root account
	let raw_private_key = MOVEMENT_CONFIG
		.execution_config
		.maptos_config
		.chain
		.maptos_private_key_signer_identifier
		.try_raw_private_key()?;
	let private_key_hex = hex::encode(raw_private_key);
	let root_account = LocalAccount::from_private_key(private_key_hex.as_str(), 0)?;

	// form the local account release signer
	let core_resources_account =
		LocalAccountReleaseSigner::new(root_account, Some(aptos_test_root_address()));

	// Now, let's rotate the key and see if the rotated account
	// can still sign off on governance proposals

	// Generate recipient account
	let recipient = LocalAccount::generate(&mut rand::rngs::OsRng);

	faucet_client.fund(recipient.address(), 100_000_000_000).await?;

	let recipient_bal = coin_client
		.get_account_balance(&recipient.address())
		.await
		.context("Failed to get recipient's account balance")?;

	let core_resource_bal = coin_client
		.get_account_balance(&core_resources_account.address())
		.await
		.context("Failed to get core resources account balance")?;

	info!("Recipient's balance: {:?}", recipient_bal);
	info!("Core Resources Account balance: {:?}", core_resource_bal);

	// --- Offer Rotation Capability ---
	let rotation_capability_proof = RotationCapabilityOfferProofChallengeV2 {
		account_address: CORE_CODE_ADDRESS,
		module_name: String::from("account"),
		struct_name: String::from("RotationCapabilityOfferProofChallengeV2"),
		chain_id: state.chain_id,
		sequence_number: core_resources_account.increment_sequence_number(),
		source_address: core_resources_account.address(),
		recipient_address: recipient.address(),
	};

	let rotation_capability_proof_msg = bcs::to_bytes(&rotation_capability_proof)
		.context("Failed to serialize rotation capability proof challenge")?;
	let rotation_proof_signed = core_resources_account
		.private_key()
		.sign_arbitrary_message(&rotation_capability_proof_msg);

	let is_valid = verify_signature(
		&core_resources_account.public_key().to_bytes(),
		&rotation_capability_proof_msg,
		&rotation_proof_signed.to_bytes(),
	)?;

	assert!(is_valid, "Signature verification failed!");
	info!("Signature successfully verified!");

	let offer_payload = make_entry_function_payload(
		CORE_CODE_ADDRESS,
		"account",
		"offer_rotation_capability",
		vec![],
		vec![
			bcs::to_bytes(&rotation_proof_signed.to_bytes().to_vec())
				.context("Failed to serialize rotation capability signature")?,
			bcs::to_bytes(&0u8).context("Failed to serialize account scheme")?,
			bcs::to_bytes(&core_resources_account.public_key().to_bytes().to_vec())
				.context("Failed to serialize public key bytes")?,
			bcs::to_bytes(&recipient.address()).context("Failed to serialize recipient address")?,
		],
	)?;

	core_resources_account.decrement_sequence_number();

	let offer_response =
		send_aptos_transaction(&rest_client, &mut core_resources_account, offer_payload).await?;
	info!("Offer transaction response: {:?}", offer_response);

	// form the rest client
	let rest_client = movement_client::rest_client::Client::new(NODE_URL.clone());

	// After completing key rotation and confirming it's successful:

	// Fetch the latest sequence number for the core resources account post-rotation
	let account_info =
		rest_client.get_account(&core_resources_account.address()).await?.into_inner();

	// Reconstruct LocalAccount using the rotated private key (recipient.private_key)
	let rotated_core_account = LocalAccount::new(
		core_resources_account.address(),
		recipient.private_key().clone(), // Rotated private key
		account_info.sequence_number,
	);

	let rotated_release_signer =
		LocalAccountReleaseSigner::new(rotated_core_account, Some(aptos_test_root_address()));

	// Finally, invoke the release flow using the rotated signer
	biarritz_rc1
		.release(&rotated_release_signer, 2_000_000, 100, 60, &rest_client)
		.await?;

	info!(
		"✅ Governance release successfully signed and executed using rotated Core Resources key."
	);

	Ok(())
}
