use alloy::node_bindings::AnvilInstance;
use aptos_sdk::crypto::ed25519::Ed25519PrivateKey;
use bridge_config::Config;
use hex::ToHex;
use tokio::process::Command;

pub mod deploy;
pub mod local;

pub async fn process_compose_setup(config: Config) -> Result<Config, anyhow::Error> {
	// Currently local only
	tracing::info!("Bridge process_compose_setup");

	let private_key_bytes: [u8; 32] = [0; 31].iter().cloned().chain([1].iter().cloned()).collect::<Vec<u8>>().try_into().unwrap();

	//config.movement.movement_signer_key = Ed25519PrivateKey::try_from(private_key_bytes.as_slice()).expect("Failed to create private key");

	//Deploy locally
	let config = crate::deploy::setup(config).await?;
	Ok(config)
}

pub async fn test_eth_setup(mut config: Config) -> Result<(Config, AnvilInstance), anyhow::Error> {
	let anvil = local::setup_eth(&mut config.eth, &mut config.testing);
	//Deploy locally
	crate::deploy::setup_local_ethereum(&mut config.eth).await?;
	Ok((config, anvil))
}

pub async fn test_mvt_setup(
	mut config: Config,
) -> Result<(Config, tokio::process::Child), anyhow::Error> {
	let movement_task = local::setup_movement_node(&mut config.movement).await?;
	deploy::deploy_local_movement_node(&mut config.movement)?;
	Ok((config, movement_task))
}

pub async fn test_suzuka_setup(
	mut config: Config,
) -> Result<(Config, tokio::process::Child), anyhow::Error> {
	let movement_task = Command::new("sleep")
		.arg("10")  // Sleep for 10 seconds
		.spawn()?;  // Spawn the process asynchronously
	let private_key_bytes: [u8; 32] = [0; 31].iter().cloned().chain([1].iter().cloned()).collect::<Vec<u8>>().try_into().unwrap();

	config.movement.movement_signer_key = Ed25519PrivateKey::try_from(private_key_bytes.as_slice()).expect("Failed to create private key");

	println!("Movement signer key before init_with_root_key: {:?}", &config.movement.movement_signer_key.to_bytes().encode_hex::<String>());
	deploy::init_with_root_key(&mut config.movement)?;
	Ok((config, movement_task))
}
