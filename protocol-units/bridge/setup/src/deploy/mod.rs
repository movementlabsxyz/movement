use alloy::node_bindings::{Anvil, AnvilInstance};
use dot_movement::DotMovement;
use bridge_config::common::bridge::Config as BridgeConfig;
use bridge_service::chains::movement::client::{MovementClient, Config as MovementConfig};
use bridge_service::chains::ethereum::client::{EthClient, Config as EthConfig};
use godfig::{backend::config_file::ConfigFile, Godfig};
use tokio::process::Child;
use tracing_subscriber::EnvFilter;

/// The local setup strategy for MCR settlement
#[derive(Debug, Clone)]
pub struct Deploy {}

impl Deploy {
	/// Instantiates the local setup strategy with ports on localhost
	/// to configure for Ethernet RPC and WebSocket client access.
	pub fn new() -> Self {
		Self {}
	}
}

impl Default for Deploy {
	fn default() -> Self {
		Deploy::new()
	}
}

impl Deploy {
        async fn setup(
                &self, 
                dot_movement: &DotMovement,
		mut config: BridgeConfig,
        ) -> Result<(AnvilInstance, Child), anyhow::Error> {
                // Initialize tracing
                tracing_subscriber::fmt()
                        .with_env_filter(
                        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
                        )
                        .init();
        
                // Get the config file
                let dot_movement = dot_movement::DotMovement::try_from_env()?;
                let config_file = dot_movement.try_get_or_create_config_file().await?;
        
                // Get a matching godfig object
                let godfig: Godfig<BridgeConfig, ConfigFile> =
                        Godfig::new(ConfigFile::new(config_file), vec!["bridge".to_string()]);
        
                let eth_config = EthConfig::build_for_test();
                let eth_client = EthClient::new(eth_config).await?;
                let anvil = Anvil::new().port(eth_client.rpc_port()).spawn();
                    
                let (movement_client, child) = MovementClient::new_for_test(MovementConfig::build_for_test()).await?; 
        
                // Run a godfig transaction to update the file
                godfig
                        .try_transaction(|config| async move {
                        println!("Config: {:?}", config);
                        let (config, _) = setup().await?;
                        Ok(Some(config))
                        })
                        .await?;
        
                Ok((anvil, child))
        }

}