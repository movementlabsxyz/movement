use anyhow::Context;
use commander::run_command;
use m1_da_light_node_util::config::local::Config;
use crate::common;
use tracing::info;
use dot_movement::DotMovement;

#[derive(Debug, Clone)]
pub struct Arabica;

impl Arabica {

    pub fn new() -> Self {
        Self
    }


    pub async fn get_arabica_11_address(&self) -> Result<String, anyhow::Error> {
        // get the json from celkey
        // cel-key list --node.type light --keyring-backend test --p2p.network arabica --output json
        let json_string = run_command(
            "cel-key",
            &[
                "list",
                "--node.type",
                "light",
                "--keyring-backend",
                "test",
                "--p2p.network",
                "arabica",
                "--output",
                "json",
            ],
        ).await?;
        
        let json_string = json_string.lines().last().context("Failed to get the last line of the json string.")?;

        info!("Arabica 11 address json: {}", json_string);

        // use serde to convert to json
        let json: serde_json::Value = serde_json::from_str(&json_string)
            .context("Failed to convert json string to json value for celestia address.")?;

        // q -r '.[0].address'
        let address = json
            .get(0)
            .context("Failed to get the first element of the json array.")?
            .get("address")
            .context("Failed to get the address field from the json object.")?
            .as_str()
            .context("Failed to convert the address field to a string.")?;

        Ok(address.to_string())

    }

    pub async fn celestia_light_init(&self) -> Result<(), anyhow::Error> {
        // celestia light init --p2p.network arabica
        run_command(
            "celestia",
            &[
                "light",
                "init",
                "--p2p.network",
                "arabica",
            ],
        ).await?;

        Ok(())
    }

    pub async fn get_da_block_height(&self) -> Result<u64, anyhow::Error> {
        let response = reqwest::get("https://rpc.celestia-arabica-11.com/block")
            .await?
            .text()
            .await?;

        Ok(response.parse().context("Failed to parse the response to a u64.")?)
    }

    pub async fn get_auth_token(&self) -> Result<String, anyhow::Error> {
        // celestia light auth admin --p2p.network arabica
        let auth_token = run_command(
            "celestia",
            &[
                "light",
                "auth",
                "admin",
                "--p2p.network",
                "arabica",
            ],
        ).await?.trim().to_string();

        Ok(auth_token)
    }

    pub async fn setup_celestia(
        &self,
        dot_movement: DotMovement,
        mut config: Config,
    ) -> Result<Config, anyhow::Error> {

        let mut config = common::celestia::initialize_celestia_config(dot_movement.clone(), config)?;
		let mut config = common::memseq::initialize_memseq_config(dot_movement.clone(), config)?;
		let mut config = common::celestia::make_dirs(dot_movement.clone(), config).await?;

        // celestia light init --p2p.network arabica
        self.celestia_light_init().await?;

        // get the arabica 11 address
        let address = self.get_arabica_11_address().await?;
        config.appd.celestia_validator_address.replace(address.clone());

        // get the auth token
        let auth_token = self.get_auth_token().await?;
        config.appd.celestia_auth_token.replace(auth_token.clone());

        // create and fund the account
        self.create_and_fund_account(dot_movement.clone(), config.clone()).await?;

        Ok(config)
    }

    pub async fn create_and_fund_account(
        &self,
		dot_movement: DotMovement,
		config: Config,
    ) -> Result<(), anyhow::Error> {
        
        /**
         * #!/bin/bash
        # Maximum number of retries
        max_retries=10
        # Delay in seconds between retries
        retry_delay=5

        retry_count=0
        success=false

        while [ $retry_count -lt $max_retries ]; do
            # Run the curl command
            response=$(curl -s -X POST 'https://faucet.celestia-arabica-11.com/api/v1/faucet/give_me' \
                -H 'Content-Type: application/json' \
                -d "{\"address\": \"$CELESTIA_ADDRESS\", \"chainId\": \"arabica-11\"}")
            
            # Process the response with jq
            txHash=$(echo "$response" | jq -e '.txHash')
            
            # Check if jq found the txHash
            if [ $? -eq 0 ]; then
                echo "Transaction hash: $txHash"
                success=true
                break
            else
                echo "Error: txHash field not found in the response." >&2
                # Increment the retry counter
                retry_count=$((retry_count+1))
                # Wait before retrying
                sleep $retry_delay
            fi
        done

        # Check if the operation was successful
        if [ "$success" = false ]; then
            echo "Failed to retrieve txHash after $max_retries attempts." >&2
        fi
         */

        let celestia_address = config.appd.celestia_validator_address.context(
            "Celestia validator address is not set in the config.",
        )?.clone();

        let max_retries = 10;
        let retry_delay = 5;
        let mut retry_count = 0;
        let mut success = false;

        while retry_count < max_retries {
            let response = reqwest::Client::new()
                .post("https://faucet.celestia-arabica-11.com/api/v1/faucet/give_me")
                .header("Content-Type", "application/json")
                .body(serde_json::json!({
                    "address": celestia_address,
                    "chainId": "arabica-11",
                }).to_string())
                .send()
                .await?
                .text()
                .await?;

            let res = serde_json::from_str::<serde_json::Value>(&response)
                .context("Failed to parse the response to a json value.")?;
            let tx_hash = res
                .get("txHash")
                .context("Failed to get the txHash field from the response.")?;

            if tx_hash.is_string() {
                let tx_hash = tx_hash.as_str().context("Failed to convert the txHash field to a string.")?;
                info!("Transaction hash: {}", tx_hash);
                success = true;
                break;
            } else {
                info!("Error: txHash field not found in the response.");
                retry_count += 1;
                tokio::time::sleep(tokio::time::Duration::from_secs(retry_delay)).await;
            }
        }

        Ok(())
    }

	pub async fn setup(
		&self,
		dot_movement: DotMovement,
		config: Config,
	) -> Result<Config, anyhow::Error> {

		// By default the M1 DA Light Node is not initialized.
		if !config.m1_da_light_node_is_initial {
			info!("M1 DA Light Node is already initialized.");
			return Ok(config);
		}

		info!("Setting up Celestia for M1 DA Light Node.");
		let mut config = self.setup_celestia(dot_movement, config).await?;

		info!("M1 DA Light Node setup complete.");

		// Now we set the config to initialized.
		config.m1_da_light_node_is_initial = false;

		// Placeholder for returning the actual configuration.
		Ok(config)
	}

}