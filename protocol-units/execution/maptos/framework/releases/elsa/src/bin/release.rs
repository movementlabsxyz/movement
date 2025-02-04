use aptos_framework_elsa_release::cached::gas_upgrade::Elsa;
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
	// form the elsa release
	let elsa = Elsa::new();

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
	let local_account_release_signer =
		LocalAccountReleaseSigner::new(root_account, Some(aptos_test_root_address()));

	// form the rest client
	let rest_client = movement_client::rest_client::Client::new(NODE_URL.clone());

	// release the elsa release
	elsa.release(
		&local_account_release_signer,
		2_000_000,
		100,
		// 60 seconds from now as u64
		60,
		&rest_client,
	)
	.await?;

	Ok(())
}
