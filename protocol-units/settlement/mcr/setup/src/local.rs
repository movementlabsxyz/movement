use super::Setup;
use alloy_primitives::hex;
use commander::run_command;
use dot_movement::DotMovement;
use k256::ecdsa::SigningKey;
use mcr_settlement_config::Config;
use rand::{thread_rng, Rng};
use std::env;
use std::future::Future;
use tracing::info;

const DEFAULT_ETH_RPC_PORT: u16 = 8545;
const DEFAULT_ETH_WS_PORT: u16 = 8545;

/// The local setup strategy for MCR settlement
#[derive(Debug, Clone)]
pub struct Local {
	eth_rpc_port: u16,
	eth_ws_port: u16,
}

impl Local {
	/// Instantiates the local setup strategy with ports on localhost
	/// to configure for Ethernet RPC and WebSocket client access.
	pub fn new(eth_rpc_port: u16, eth_ws_port: u16) -> Self {
		Self { eth_rpc_port, eth_ws_port }
	}
}

impl Default for Local {
	fn default() -> Self {
		Local::new(DEFAULT_ETH_RPC_PORT, DEFAULT_ETH_WS_PORT)
	}
}

impl Setup for Local {
	fn setup(
		&self,
		_dot_movement: &DotMovement,
		mut config: Config,
	) -> impl Future<Output = Result<Config, anyhow::Error>> + Send {
		//define a temporary chain Id for Anvil
		let mut rng = thread_rng(); // rng is not send.
		let id: u16 = rng.gen_range(100, 32768);
		let chain_id = id.to_string();

		tracing::info!("Init Settlement local conf");

		async move {
			if config.rpc_url.is_none() {
				config.rpc_url = Some(format!("http://localhost:{}", self.eth_rpc_port));
			}
			if config.ws_url.is_none() {
				config.ws_url = Some(format!("http://localhost:{}", self.eth_ws_port));
			}

			tracing::info!("Run Settlement local conf");
			//start local process and deploy smart contract.

			//define working directory of Anvil
			let mut path = std::env::current_dir()?;
			let storage_path = env::var("MOVEMENT_BASE_STORAGE_PATH").unwrap_or_else(|_| {
				tracing::info!(
					"MOVEMENT_BASE_STORAGE_PATH not set. Use the default value: .movement",
				);
				".movement".to_string()
			});
			path.push(storage_path);
			path.push("anvil/mcr");
			path.push(chain_id.clone());
			tokio::fs::create_dir_all(&path).await?;
			tracing::info!("dir exist:{}", path.exists());
			path.push("anvil.json");

			let anvil_path = path.to_string_lossy().to_string();
			tracing::info!("anvil_path: {:?}", anvil_path);

			tokio::spawn({
				let anvil_path = anvil_path.clone();
				let chain_id = chain_id.clone();
				async move {
					let result = run_command(
						"anvil",
						&[
							"--chain-id",
							&chain_id,
							"--config-out",
							&anvil_path,
							"--port",
							&DEFAULT_ETH_RPC_PORT.to_string(),
						],
					)
					.await;
					tracing::info!("Anvil start result:{result:?}");
				}
			});

			//wait Anvil to start
			let _ = tokio::time::sleep(tokio::time::Duration::from_millis(2000)).await;
			tracing::info!("after start:{}", path.exists());

			// Deploy MCR smart contract.
			let anvil_addresses =
				mcr_settlement_client::eth_client::read_anvil_json_file_addresses(&*anvil_path)?;
			let settlement_private_key = &anvil_addresses[1].private_key;
			let settlement_address = &anvil_addresses[1].address;

			let mut solidity_path = std::env::current_dir()?;
			solidity_path.push("protocol-units/settlement/mcr/contracts");
			let solidity_path = solidity_path.to_string_lossy();
			tracing::info!("solidity_path: {:?}", solidity_path);
			let mcr_address = run_command(
				"forge",
				&[
					"script",
					"DeployMCRLegacy",
					"--root",
					&solidity_path,
					"--broadcast",
					"--chain-id",
					&chain_id,
					"--sender",
					&settlement_address,
					"--rpc-url",
					&config.rpc_url.clone().unwrap(),
					"--private-key",
					&settlement_private_key,
				],
			)
			.await?
			.trim()
			.to_string();

			// Can't use rand 0.7 while k256 is on rand 0.6
			let mut rng = k256::elliptic_curve::rand_core::OsRng;

			info!("setting up MCR Ethereum client mcr_address:{mcr_address}");

			if config.signer_private_key.is_none() {
				let key = SigningKey::random(&mut rng);
				let key_bytes = key.to_bytes();
				let private_key = hex::encode(key_bytes.as_slice());
				config.signer_private_key = Some(private_key);
			}

			Ok(config)
		}
	}
}
