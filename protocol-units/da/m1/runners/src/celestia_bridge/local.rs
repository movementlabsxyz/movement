use crate::Runner;
use anyhow::Result;
use reqwest::Client;
use serde_json::Value;
use std::time::Duration;
use tokio::time::sleep;
use tracing::info;

#[derive(Debug, Clone)]
pub struct Local;

impl Local {
    pub fn new() -> Self {
        Local
    }

    async fn get_genesis_block(&self) -> Result<String> {
        let client = Client::new();
        let mut genesis = String::new();
        let mut cnt = 0;
        let max_attempts = 30;

        while genesis.len() <= 4 && cnt < max_attempts {
            info!("Waiting for genesis block.");
            let response = client.get("http://127.0.0.1:26657/block?height=1")
                .send()
                .await?
                .text()
                .await?;
            let json: Value = serde_json::from_str(&response)?;
            genesis = json["result"]["block_id"]["hash"]
                .as_str()
                .unwrap_or("")
                .to_string();
            info!("Genesis: {}", genesis);
            cnt += 1;
            sleep(Duration::from_secs(1)).await;
            info!("Attempt {}", cnt);
        }

        if genesis.len() <= 4 {
            info!(
                "Failed to retrieve genesis block after {} attempts.",
                max_attempts
            );
            return Err(anyhow::anyhow!(
                "Failed to retrieve genesis block after maximum attempts"
            ));
        }

        info!("Discovered genesis: {}", genesis);
        Ok(genesis)
    }
}

impl Runner for Local {
    async fn run(
        &self,
        dot_movement: dot_movement::DotMovement,
        config: m1_da_light_node_util::Config,
    ) -> Result<()> {

        let genesis = self.get_genesis_block().await?;

        let node_store = config.try_celestia_node_path()?;
        info!("Initializing Celestia Bridge with node store at {}", node_store);
        // celestia bridge init --node.store $CELESTIA_NODE_PATH
        commander::run_command(
            "celestia-bridge",
            &[
                "init",
                "--node.store", &node_store,
            ],
        ).await?;

        info!("Starting celestia-bridge.");
        // celestia bridge start \
        // --node.store $CELESTIA_NODE_PATH --gateway \
        // --core.ip 0.0.0.0 \
        // --keyring.accname validator \
        // --gateway.addr 0.0.0.0 \
        // --rpc.addr 0.0.0.0 \
        // --log.level $CELESTIA_LOG_LEVEL
        commander::run_command(
            "celestia-bridge",
            &[
                "start",
                "--node.store", &config.try_celestia_node_path()?,
                "--gateway",
                "--core.ip", "0.0.0.0",
                "--keyring.accname", "validator",
                "--gateway.addr", "0.0.0.0",
                "--rpc.addr", "0.0.0.0"
            ],
        ).await?;
        

        Ok(())
    }
}
