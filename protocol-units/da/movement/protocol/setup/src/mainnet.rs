use crate::common;
use anyhow::Context;
use commander::run_command;
use dot_movement::DotMovement;
use movement_da_util::config::Config;
use tracing::info;

#[derive(Debug, Clone)]
pub struct Mainnet;

impl Mainnet {
	pub fn new() -> Self {
		Self
	}

	pub async fn celestia_light_init(&self) -> Result<(), anyhow::Error> {
		// celestia light init --p2p.network celestia --keyring.backend test
		run_command(
			"celestia",
			&["light", "init", "--p2p.network", "celestia", "--keyring.backend", "test"],
		)
		.await?;

		Ok(())
	}

	pub async fn get_da_block_height(&self) -> Result<u64, anyhow::Error> {
		let response = reqwest::get("https://rpc.celestia.pops.one/block").await?.text().await?;

		Ok(response.parse().context("Failed to parse the response to a u64.")?)
	}

	pub async fn get_auth_token(&self) -> Result<String, anyhow::Error> {
		// celestia light auth admin --p2p.network celestia
		let auth_token =
			run_command("celestia", &["light", "auth", "admin", "--p2p.network", "celestia"])
				.await?
				.trim()
				.to_string();

		Ok(auth_token)
	}

	pub async fn setup_celestia(
		&self,
		dot_movement: DotMovement,
		config: Config,
	) -> Result<Config, anyhow::Error> {
		let config = common::celestia::initialize_celestia_config(dot_movement.clone(), config)?;
		let config = common::memseq::initialize_memseq_config(dot_movement.clone(), config)?;
		let mut config = common::celestia::make_dirs(dot_movement.clone(), config).await?;

		// celestia light init --p2p.network celestia
		self.celestia_light_init().await?;

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
		if !config.da_light_node_is_initial {
			info!("M1 DA Light Node is already initialized.");
			return Ok(config);
		}

		info!("Setting up Celestia for M1 DA Light Node.");
		let mut config = self.setup_celestia(dot_movement, config).await?;

		info!("M1 DA Light Node setup complete.");

		// Now we set the config to initialized.
		config.da_light_node_is_initial = false;

		// Placeholder for returning the actual configuration.
		Ok(config)
	}
}
