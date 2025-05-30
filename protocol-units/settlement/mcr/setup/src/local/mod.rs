use anyhow::{anyhow, Context};
use commander::run_command;
use dot_movement::DotMovement;
use mcr_settlement_config::Config;
use movement_signer::key::Key;
use movement_signer::key::TryFromCanonicalString;
use movement_signer_loader::identifiers::aws_kms::AwsKms;
use movement_signer_loader::identifiers::{local::Local as SignerLocal, SignerIdentifier};
use tracing::info;

/// The local setup strategy for MCR settlement
#[derive(Debug, Clone)]
pub struct Local {}

impl Local {
	/// Instantiates the local setup strategy with ports on localhost
	/// to configure for Ethernet RPC and WebSocket client access.
	pub fn new() -> Self {
		Self {}
	}
}

impl Default for Local {
	fn default() -> Self {
		Local::new()
	}
}

impl Local {
	pub async fn setup(
		&self,
		dot_movement: &DotMovement,
		mut config: Config,
	) -> Result<(Config, tokio::task::JoinHandle<Result<String, anyhow::Error>>), anyhow::Error> {
		println!("local setup config {config:?}",);

		let chain_id = 3073;
		config.eth_connection.eth_chain_id = chain_id;

		tracing::info!("Init Settlement local conf");

		//start local process and deploy smart contract.
		//define working directory of Anvil
		info!("Starting Anvil");
		let mut path = dot_movement.get_path().to_path_buf();
		path.push("anvil/mcr");
		path.push(chain_id.to_string().clone());
		tokio::fs::create_dir_all(&path)
			.await
			.context("Failed to create Anvil directory")
			.context("Failed to create Anvil directory")?;
		path.push("anvil.json");

		let exists = tokio::fs::try_exists(&path)
			.await
			.context("Failed to check if Anvil file exists")?;

		info!(
			"Anvil path: `{}`, {}",
			path.display(),
			if exists { "exists" } else { "does not exist" }
		);

		let anvil_path = path.to_string_lossy().to_string();

		let config_clone = config.clone();
		let anvil_path_clone = anvil_path.clone();
		let anvil_join_handle = tokio::task::spawn(async move {
			run_command(
				"anvil",
				&vec![
					"--chain-id",
					&config_clone.eth_connection.eth_chain_id.to_string(),
					"--config-out",
					&anvil_path_clone,
					"--port",
					&config_clone.eth_connection.eth_rpc_connection_port.to_string(),
					"--host",
					"0.0.0.0",
				],
			)
			.await
			.context("Failed to start Anvil")
		});

		//wait Anvil to start
		let mut counter = 0;
		loop {
			if counter > 100 {
				return Err(anyhow!("Anvil didn't start in time"));
			}
			counter += 1;
			if path.exists() {
				break;
			}
			let _ = tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
		}

		let anvil_addresses =
			mcr_settlement_client::eth_client::read_anvil_json_file_addresses(&*anvil_path)
				.context("Failed to read Anvil addresses")?;
		if let Some(deploy) = &mut config.deploy {
			let private_key_hex = anvil_addresses
				.get(0)
				.ok_or(anyhow!("Failed to get Anvil address"))?
				.private_key
				.clone();
			let private_key_hex_bytes = (&private_key_hex[2..]).to_string();

			info!("Use Local Signer.");
			deploy.signer_identifier =
				SignerIdentifier::Local(SignerLocal { private_key_hex_bytes });

			let deployer_address = anvil_addresses
				.get(0)
				.ok_or(anyhow!("Failed to get Anvil address"))?
				.address
				.clone();
			info!("Deployer address: {}", deployer_address);

			// Detect if we execute with AWS from AWS_KMS_KEY_ID env var or not.
			config.settle.signer_identifier = match std::env::var("AWS_KMS_KEY_ID") {
				Ok(key_id) => {
					info!("Use AWS Signer with key_id: {key_id}");
					//For Aws set identifier to AWS and key_id
					SignerIdentifier::AwsKms(AwsKms {
						key: Key::try_from_canonical_string(&format!(
							"movement/dev/full_node/mcr_settlement/signer/{}/0",
							key_id
						))
						.map_err(|err| anyhow::anyhow!(err))?,
						// todo: for non-local setups or where the signer is hardcoded, already configured, etc., we should not create the key
						create: true,
					})
				}
				Err(_) => deploy.signer_identifier.clone(),
			};
		}
		if let Some(testing) = &mut config.testing {
			// Remove the old one if the config was existing.
			testing.well_known_account_private_keys.clear();
			for anvil_address in &anvil_addresses {
				testing.well_known_account_private_keys.push(anvil_address.private_key.clone());
			}
		}

		Ok((config, anvil_join_handle))
	}
}
