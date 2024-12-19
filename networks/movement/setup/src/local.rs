use crate::MovementFullNodeSetupOperations;
use aptos_sdk::crypto::ed25519::Ed25519PublicKey;
use aptos_sdk::types::transaction::authenticator::AuthenticationKey;
use dot_movement::DotMovement;

// use tracing::debug;

#[derive(Debug, Clone, Default)]
pub struct Local {
	mcr_settlement_strategy: mcr_settlement_setup::Setup,
}

impl Local {
	pub fn new() -> Self {
		Self { mcr_settlement_strategy: Default::default() }
	}

	async fn run_da_light_node_setup(
		&self,
		dot_movement: DotMovement,
		mut config: movement_config::Config,
	) -> Result<
		(movement_config::Config, tokio::task::JoinHandle<Result<String, anyhow::Error>>),
		anyhow::Error,
	> {
		let da_light_node_config = config.celestia_da_light_node.clone();

		let new_da_light_node_config = movement_celestia_da_light_node_setup::setup(
			dot_movement.clone(),
			da_light_node_config,
		)
		.await?;

		// Update the config with the new da_light_node_config
		config.celestia_da_light_node = new_da_light_node_config;

		tracing::info!("Running mcr_settlement_setup");
		let mcr_settlement_config: mcr_settlement_config::Config = config.mcr.clone();
		let (mcr_config, join_handle) =
			self.mcr_settlement_strategy.setup(&dot_movement, mcr_settlement_config).await?;
		config.mcr = mcr_config;

		Ok((config, join_handle))
	}

	async fn setup_maptos_execution_config(
		&self,
		dot_movement: DotMovement,
		mut config: movement_config::Config,
	) -> Result<movement_config::Config, anyhow::Error> {
		// update the db path
		let chain_id = config.execution_config.maptos_config.chain.maptos_chain_id;
		let db_path = dot_movement
			.get_path()
			.join("maptos")
			.join(chain_id.to_string())
			.join(".maptos");
		config.execution_config.maptos_config.chain.maptos_db_path.replace(db_path);

		// write the maptos signer address to the default signer address whitelist
		let default_signer_address_whitelist_path =
			dot_movement.get_path().join("default_signer_address_whitelist");

		let signer_account_address = AuthenticationKey::ed25519(&Ed25519PublicKey::from(
			&config.execution_config.maptos_config.chain.maptos_private_key,
		))
		.account_address();

		std::fs::write(
			default_signer_address_whitelist_path.clone(),
			format!(
				"{}\n{}",
				signer_account_address.to_hex(),
				"000000000000000000000000000000000000000000000000000000000a550c18"
			),
		)?;

		Ok(config)
	}

	async fn setup_da_db_config(
		&self,
		dot_movement: DotMovement,
		mut config: movement_config::Config,
	) -> Result<movement_config::Config, anyhow::Error> {
		// update the db path
		let db_path = dot_movement.get_path().join(config.da_db.da_db_path.clone());
		config.da_db.da_db_path = db_path
			.to_str()
			.ok_or(anyhow::anyhow!("Failed to convert db path to string: {:?}", db_path))?
			.to_string();

		Ok(config)
	}
}

impl MovementFullNodeSetupOperations for Local {
	async fn setup(
		&self,
		dot_movement: DotMovement,
		config: movement_config::Config,
	) -> Result<
		(movement_config::Config, tokio::task::JoinHandle<Result<String, anyhow::Error>>),
		anyhow::Error,
	> {
		// Run the DA light node setup
		let (config, join_handle) =
			self.run_da_light_node_setup(dot_movement.clone(), config).await?;

		// run the maptos execution config setup
		let config = self.setup_maptos_execution_config(dot_movement.clone(), config).await?;

		// run the da_db config setup
		let config = self.setup_da_db_config(dot_movement.clone(), config).await?;

		// Placeholder for returning the actual configuration.
		Ok((config, join_handle))
	}
}
