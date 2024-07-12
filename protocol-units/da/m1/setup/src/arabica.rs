use anyhow::Context;
use commander::run_command;
use m1_da_light_node_util::config::local::Config;
use crate::common;

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

        // use serde to convert to json
        let json: serde_json::Value = serde_json::from_str(&json_string)
            .context("Failed to convert json string to json value.")?;

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

        // get the block height
        let block_height = self.get_da_block_height().await?;
        config.appd.da_block_height.replace(block_height);

        // get the auth token
        let auth_token = self.get_auth_token().await?;
        config.appd.celestia_auth_token.replace(auth_token.clone());

        Ok(config)
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