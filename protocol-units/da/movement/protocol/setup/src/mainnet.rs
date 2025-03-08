use crate::common;
use dot_movement::DotMovement;
use movement_da_util::config::Config;
use tracing::info;

#[derive(Debug, Clone)]
pub struct Mainnet;

impl Mainnet {
	pub fn new() -> Self {
		Self
	}

	async fn celestia_light_init(
		&self,
		dot_movement: &DotMovement,
		config: &Config,
	) -> Result<(), anyhow::Error> {
		common::celestia::celestia_light_init(dot_movement, config, "celestia").await
	}

	pub async fn get_da_block_height(&self) -> Result<u64, anyhow::Error> {
		common::celestia::current_block_height("https://rpc.celestia.pops.one").await
	}

	async fn get_auth_token(
		&self,
		dot_movement: &DotMovement,
		config: &Config,
	) -> Result<String, anyhow::Error> {
		common::celestia::get_auth_token(dot_movement, config, "celestia").await
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
		self.celestia_light_init(&dot_movement, &config).await?;

		// get the auth token
		let auth_token = self.get_auth_token(&dot_movement, &config).await?;
		config.appd.celestia_auth_token.replace(auth_token.clone());

		// get the initial block height
		config.initial_height = self.get_da_block_height().await?;

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
