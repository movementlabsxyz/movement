use aptos_framework_post_l1_merge_release::vote::full_governance_vote;
use aptos_framework_post_l1_merge_release::{
	cached::full::feature_upgrade::PostL1Merge, vote::test_partial_vote,
};
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

mod governance;

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

	let post_l1_release = PostL1Merge::new();

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

	full_governance_vote();

	post_l1_release
		.release(&local_account_release_signer, 2_000_000, 100, 60, &rest_client)
		.await?;

	Ok(())
}
