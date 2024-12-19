use crate::common;
use anyhow::Context;
use commander::run_command;
use dot_movement::DotMovement;
use movement_celestia_da_util::config::local::Config;
use tokio::fs;
use tracing::info;

#[derive(Debug, Clone)]
pub struct Local;

impl Local {
	pub fn new() -> Self {
		Self
	}

	async fn setup_celestia(
		&self,
		dot_movement: DotMovement,
		config: Config,
	) -> Result<Config, anyhow::Error> {
		info!("Setting up Celestia.");
		let config = common::celestia::initialize_celestia_config(dot_movement.clone(), config)?;
		let config = common::memseq::initialize_memseq_config(dot_movement.clone(), config)?;
		let mut config = common::celestia::make_dirs(dot_movement.clone(), config).await?;
		info!("Setup config for Memseq and Celestia: {:?}", config);

		// unpack some of the config values
		let celestia_chain_id = config.appd.celestia_chain_id.clone();
		info!("Setting up Celestia for chain id: {}", celestia_chain_id);
		let celestia_app_path = config.appd.celestia_path.clone().context(
			"Failed to get Celestia App path from config. This is required for setting up Celestia.",
		)?;
		info!("Celestia App Path: {}", celestia_app_path);
		let celestia_node_path = config.bridge.celestia_bridge_path.clone().context(
			"Failed to get Celestia Node path from config. This is required for setting up Celestia.",
		)?;
		info!("Celestia Node Path: {}", celestia_node_path);

		// initialize the celestia app
		info!("Initializing the Celestia App.");
		run_command(
			"celestia-appd",
			&[
				"init",
				&celestia_chain_id,
				"--chain-id",
				&celestia_chain_id,
				"--home",
				&celestia_app_path,
			],
		)
		.await?;

		// add the validator key
		info!("Adding the validator key.");
		run_command(
			"celestia-appd",
			&["keys", "add", "validator", "--keyring-backend=test", "--home", &celestia_app_path],
		)
		.await?;

		// get the validator address
		info!("Getting the validator address.");
		let validator_address = run_command(
			"celestia-appd",
			&[
				"keys",
				"show",
				"validator",
				"-a",
				"--keyring-backend=test",
				"--home",
				&celestia_app_path,
			],
		)
		.await?
		.trim()
		.to_string();
		config.appd.celestia_validator_address.replace(validator_address.clone());

		// add the genesis account
		info!("Adding the genesis account.");
		let coins = "1000000000000000utia";
		run_command(
			"celestia-appd",
			&["add-genesis-account", &validator_address, coins, "--home", &celestia_app_path],
		)
		.await?;

		// create the genesis transaction
		info!("Creating the genesis transaction.");
		run_command(
			"celestia-appd",
			&[
				"gentx",
				"validator",
				"5000000000utia",
				"--fees",
				"1utia",
				"--keyring-backend=test",
				"--chain-id",
				&celestia_chain_id,
				"--home",
				&celestia_app_path,
			],
		)
		.await?;

		// collect the genesis transactions
		info!("Collecting the genesis transactions.");
		run_command("celestia-appd", &["collect-gentxs", "--home", &celestia_app_path]).await?;

		// update the celestia node config
		info!("Updating the Celestia Node config.");
		self.update_celestia_node_config(&celestia_app_path).await?;

		// copy the keys over
		info!("Copying keys from Celestia App to Celestia Node.");
		self.copy_keys(&celestia_app_path, &celestia_node_path).await?;

		// get the auth token
		// celestia bridge auth admin --node.store ${CELESTIA_NODE_PATH}
		info!("Getting the auth token.");
		let auth_token = run_command(
			"celestia",
			&[
				"bridge",
				"auth",
				"admin",
				"--node.store",
				&celestia_node_path,
				"--keyring.backend",
				"test",
			],
		)
		.await?
		.trim()
		.to_string();
		config.appd.celestia_auth_token.replace(auth_token.clone());

		info!("Celestia setup complete.");

		Ok(config)
	}

	/// Updates the Celestia Node config
	async fn update_celestia_node_config(&self, home: &str) -> Result<(), anyhow::Error> {
		let config_path = format!("{}/config/config.toml", home);
		let sed_commands = [
			("s#\"tcp://0.0.0.0:26657\"#\"tcp://0.0.0.0:26657\"#g", &config_path),
			("s/^timeout_commit\\s*=.*/timeout_commit = \"2s\"/g", &config_path),
			("s/^timeout_propose\\s*=.*/timeout_propose = \"2s\"/g", &config_path),
		];

		for (command, path) in &sed_commands {
			run_command("sed", &["-i.bak", command, path]).await?;
		}

		Ok(())
	}

	/// Copies keys from Celestia App to Celestia Node
	async fn copy_keys(&self, app_path: &str, node_path: &str) -> Result<(), anyhow::Error> {
		let keyring_source = format!("{}/keyring-test/", app_path);
		let keyring_dest = format!("{}/keys/keyring-test/", node_path);

		fs::create_dir_all(&format!("{}/keys", node_path)).await?;
		self.copy_recursive(&keyring_source, &keyring_dest).await?;

		Ok(())
	}

	/// Recursively copies files from one directory to another
	#[async_recursion::async_recursion]
	async fn copy_recursive(&self, from: &str, to: &str) -> Result<(), anyhow::Error> {
		fs::create_dir_all(to).await?;
		let mut dir = fs::read_dir(from).await?;
		while let Some(entry) = dir.next_entry().await? {
			let entry_path = entry.path();
			let dest_path = format!("{}/{}", to, entry.file_name().to_string_lossy());
			if entry_path.is_dir() {
				self.copy_recursive(&entry_path.to_string_lossy(), &dest_path).await?;
			} else {
				fs::copy(&entry_path, &dest_path).await?;
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
