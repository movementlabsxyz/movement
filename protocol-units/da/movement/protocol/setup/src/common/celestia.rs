use crate::common;
use anyhow::Context;
use celestia_types::nmt::Namespace;
use commander::Run;
use dot_movement::DotMovement;
use movement_da_util::config::Config;
use rand::Rng;
use tokio::process::Command;
use tracing::info;

use std::path::PathBuf;

fn random_10_bytes() -> [u8; 10] {
	let mut rng = rand::thread_rng();
	rng.gen()
}

pub fn random_chain_id() -> String {
	hex::encode(random_10_bytes())
}

pub fn random_namespace() -> Namespace {
	Namespace::new_v0(&random_10_bytes()).unwrap()
}

fn celestia_chain_dir(dot_movement: &DotMovement, config: &Config) -> PathBuf {
	dot_movement.get_path().join("celestia").join(&config.appd.celestia_chain_id)
}

pub fn initialize_celestia_config(
	dot_movement: DotMovement,
	mut config: Config,
) -> Result<Config, anyhow::Error> {
	if config.celestia_force_new_chain {
		// if forced just replace the chain id with a random one
		config.appd.celestia_chain_id = random_chain_id();
		config.appd.celestia_namespace = random_namespace();
		config.appd.celestia_chain_id.clone()
	} else {
		// if new chain is not forced, use the one in the config
		config.appd.celestia_chain_id.clone()
	};

	// set the node store directory accordingly to the chain id
	config
		.light
		.node_store
		.replace(celestia_chain_dir(&dot_movement, &config).join(".celestia-light"));

	// update the app path with the chain id
	config.appd.celestia_path.replace(
		celestia_chain_dir(&dot_movement, &config)
			.join(".celestia-app")
			.to_str()
			.ok_or(anyhow::anyhow!("Failed to convert path to string."))?
			.to_string(),
	);

	// update the node path with the chain id
	config.bridge.celestia_bridge_path.replace(
		celestia_chain_dir(&dot_movement, &config)
			.join(".celestia-node")
			.to_str()
			.ok_or(anyhow::anyhow!("Failed to convert path to string."))?
			.to_string(),
	);

	Ok(config)
}

pub async fn celestia_light_init(
	_dot_movement: &DotMovement,
	config: &Config,
	network: &str,
) -> Result<(), anyhow::Error> {
	// celestia light init --p2p.network <network> --keyring.backend test --node_store <dir>
	let mut command = Command::new("celestia");
	command.args(["light", "init", "--p2p.network", network, "--keyring.backend", "test"]);
	if let Some(path) = &config.light.node_store {
		command.arg("--node.store");
		command.arg(path);
	}
	// FIXME: the output does not need to be captured
	command.run_and_capture_output().await?;

	Ok(())
}

pub async fn get_auth_token(
	_dot_movement: &DotMovement,
	config: &Config,
	network: &str,
) -> Result<String, anyhow::Error> {
	// celestia light auth admin --p2p.network mocha --node.store <dir>
	let mut command = Command::new("celestia");
	command.args(["light", "auth", "admin", "--p2p.network", network]);
	if let Some(path) = &config.light.node_store {
		command.arg("--node.store");
		command.arg(path);
	}
	let auth_token = command.run_and_capture_output().await?.trim().to_string();

	Ok(auth_token)
}

pub async fn make_dirs(
	_dot_movement: DotMovement,
	config: Config,
) -> Result<Config, anyhow::Error> {
	// make the celestia app directory
	let app_path = config.appd.celestia_path.clone().context(
        "Failed to get Celestia App path from config. This is required for creating the Celestia App directory.",
    )?;
	info!("Creating Celestia App Path: {}", app_path.as_str());
	common::file::make_parent_dirs(app_path.as_str()).await?;

	// make the celestia node directory
	let node_path = config.bridge.celestia_bridge_path.clone().context(
        "Failed to get Celestia Node path from config. This is required for creating the Celestia Node directory.",
    )?;
	info!("Creating Celestia Node Path: {}", node_path.as_str());
	common::file::make_parent_dirs(node_path.as_str()).await?;

	// make the memseq database directory
	let database_path = config.memseq.sequencer_database_path.clone().context(
        "Failed to get MemSeq database path from config. This is required for creating the MemSeq database directory.",
    )?;
	info!("Creating MemSeq Database Path: {}", database_path.as_str());
	common::file::make_parent_dirs(database_path.as_str()).await?;

	Ok(config)
}

/// Retrieves the current block height from Celestia RPC
pub async fn current_block_height(rpc: &str) -> Result<u64, anyhow::Error> {
	// Request the Tendermint JSON-RPC header endpoint
	let response = reqwest::get(String::from(rpc) + "/header").await?.text().await?;

	// use serde to convert to json
	let json: serde_json::Value =
		serde_json::from_str(&response).context("Failed to parse header response as JSON")?;

	let jsonrpc_version = json
		.get("jsonrpc")
		.context("response is not JSON-RPC")?
		.as_str()
		.context("invalid jsonrpc field value")?;
	if jsonrpc_version != "2.0" {
		anyhow::bail!("unexpected JSON-RPC version {jsonrpc_version}");
	}

	// .result.header.height
	let height = json
		.get("result")
		.context("no result field")?
		.get("header")
		.context("missing header field")?
		.get("height")
		.context("missing height field")?
		.as_str()
		.context("expected string value of the height field")?
		.parse()?;
	Ok(height)
}
