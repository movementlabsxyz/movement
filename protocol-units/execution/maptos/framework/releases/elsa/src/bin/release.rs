use anyhow::Context;
use aptos_framework_elsa_release::Elsa;
use maptos_framework_release_util::Release;
use movement_client::{
	crypto::ValidCryptoMaterialStringExt, types::account_address::AccountAddress,
	types::account_config::aptos_test_root_address, types::LocalAccount,
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
	// form the elsa release bundle
	let elsa = Elsa::new();
	let release_bundle = elsa.release()?;

	// write it out the script to a temp directory
	let temp_dir = tempfile::tempdir().context("failed to create temp directory")?;
	for release_package in release_bundle.packages {
		// add a temp path in the temp directory with the .move extension
		// let temp_path = temp_dir.path().join(format!("{}.move", release_package.name()));

		// use the path working directory/proposals/script_name.move
		let temp_path = temp_dir.path().join(format!("{}.proposal.move", release_package.name()));

		release_package.generate_script_proposal_testnet(aptos_test_root_address(), temp_path)?;
	}

	let mut genesis = LocalAccount::from_private_key(
		MOVEMENT_CONFIG
			.execution_config
			.maptos_config
			.chain
			.maptos_private_key
			.to_encoded_string()?
			.as_str(),
		0,
	)?;

	Ok(())
}
