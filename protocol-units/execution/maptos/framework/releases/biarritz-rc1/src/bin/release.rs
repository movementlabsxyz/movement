use aptos_framework_biarritz_rc1_release::cached::gas_upgrade::BiarritzRc1;
use maptos_framework_release_util::{LocalAccountReleaseSigner, Release};
use movement_client::{
	crypto::ValidCryptoMaterialStringExt,
	types::{account_config::aptos_test_root_address, LocalAccount},
};
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
	let biarritz_rc1 = BiarritzRc1::new();

	// get the root account
	let root_account = LocalAccount::from_private_key(
		MOVEMENT_CONFIG
			.execution_config
			.maptos_config
			.chain
			.maptos_private_key
			.to_encoded_string()?
			.as_str(),
		0,
	)?;

	// form the local account release signer
	let local_account_release_signer =
		LocalAccountReleaseSigner::new(root_account, Some(aptos_test_root_address()));

	// form the rest client
	let rest_client = movement_client::rest_client::Client::new(NODE_URL.clone());

	// get the current sequence number
	let account = rest_client.get_account(aptos_test_root_address()).await?;
	let sequencer_number = account.into_inner().sequence_number;

	// release the elsa release
	biarritz_rc1
		.release(
			&local_account_release_signer,
			sequencer_number,
			2_000_000,
			100,
			// 60 seconds from now as u64
			((std::time::SystemTime::now()
				.checked_add(std::time::Duration::from_secs(60))
				.unwrap()
				.duration_since(std::time::UNIX_EPOCH)
				.unwrap()
				.as_secs()) as u64)
				.into(),
			MOVEMENT_CONFIG.execution_config.maptos_config.chain.maptos_chain_id,
			&rest_client,
		)
		.await?;

	Ok(())
}
