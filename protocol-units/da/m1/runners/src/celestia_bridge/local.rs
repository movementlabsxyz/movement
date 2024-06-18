use anyhow::{Context, Result};
use reqwest::Client;
use serde_json::Value;
use std::{env, time::Duration};
use tokio::time::sleep;
use tracing::info;

#[derive(Debug, Clone)]
pub struct Local;

impl Local {
	pub fn new() -> Self {
		Local
	}

	async fn get_genesis_block(
		&self,
		config: &m1_da_light_node_util::config::local::Config,
	) -> Result<String> {
		let client = Client::new();
		let mut genesis = String::new();
		let mut cnt = 0;
		let max_attempts = 30;

		// get the required connection details from the config
		let connection_hostname = config.bridge.celestia_rpc_connection_hostname.clone();
		let connection_port = config.bridge.celestia_rpc_connection_port.clone();
		let celestia_rpc_address = format!("{}:{}", connection_hostname, connection_port);

		let first_block_request_url = format!("http://{}/block?height=1", celestia_rpc_address);
		while genesis.len() <= 4 && cnt < max_attempts {
			info!("Waiting for genesis block.");
			let response = client
				.get(first_block_request_url.as_str())
				.send()
				.await?
				.text()
				.await
				.context("Failed to get genesis block from m1-da-light-node bridge runner.")?;
			let json: Value = serde_json::from_str(&response)?;
			genesis = json["result"]["block_id"]["hash"].as_str().unwrap_or("").to_string();
			info!("Genesis: {}", genesis);
			cnt += 1;
			sleep(Duration::from_secs(1)).await;
			info!("Attempt {}", cnt);
		}

		if genesis.len() <= 4 {
			info!("Failed to retrieve genesis block after {} attempts.", max_attempts);
			return Err(anyhow::anyhow!("Failed to retrieve genesis block after maximum attempts"));
		}

		info!("Discovered genesis: {}", genesis);
		Ok(genesis)
	}

	pub async fn run(
		&self,
		dot_movement: dot_movement::DotMovement,
		config: m1_da_light_node_util::config::local::Config,
	) -> Result<()> {
		let genesis = self.get_genesis_block(&config).await?;

		let node_store = config.bridge.celestia_bridge_path.clone().context(
			"Failed to get Celestia node store path from config. This is required for initializing Celestia bridge.",
		)?;
		info!("Initializing Celestia Bridge with node store at {}", node_store);
		// celestia bridge init --node.store $CELESTIA_NODE_PATH
		commander::run_command("celestia", &["bridge", "init", "--node.store", &node_store])
			.await?;

		info!("Starting celestia bridge.");
		if config.bridge.celestia_bridge_use_replace_args {
			// Convert Vec<String> to Vec<&str>
			let args: Vec<&str> = config
				.bridge
				.celestia_bridge_replace_args
				.iter()
				.map(|arg| arg.as_str())
				.collect();

			// Convert Vec<&str> to &[&str]
			let args_slice: &[&str] = &args;

			commander::run_command("celestia", args_slice).await?;
			return Ok(());
		}

		// celestia bridge start \
		// --node.store $CELESTIA_NODE_PATH --gateway \
		// --core.ip 0.0.0.0 \
		// --keyring.accname validator \
		// --gateway.addr 0.0.0.0 \
		// --rpc.addr 0.0.0.0 \
		// --log.level $CELESTIA_LOG_LEVEL
		let chain_id = config.appd.celestia_chain_id.clone();
		let celestia_custom = format!("{}:{}", &chain_id, &genesis);
		env::set_var("CELESTIA_CUSTOM", celestia_custom);
		commander::run_command(
			"celestia",
			&[
				"bridge",
				"start",
				"--node.store",
				&node_store,
				"--gateway",
				"--core.ip",
				&config.bridge.celestia_websocket_listen_hostname,
				"--keyring.accname",
				"validator",
				"--gateway.addr",
				"0.0.0.0",
				"--rpc.addr",
				&config.bridge.celestia_websocket_listen_hostname,
			],
		)
		.await?;

		Ok(())
	}
}
