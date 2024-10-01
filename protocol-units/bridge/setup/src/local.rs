use alloy::node_bindings::{Anvil, AnvilInstance};
use bridge_config::common::bridge::Config as BridgeConfig;
use bridge_service::chains::ethereum::client::{Config as EthConfig, EthClient};
use bridge_service::chains::movement::client::{Config as MovementConfig, MovementClient};
use dot_movement::DotMovement;
use godfig::{backend::config_file::ConfigFile, Godfig};
use tokio::process::Child;
use tracing_subscriber::EnvFilter;

/// The local setup strategy for the Bridge
#[derive(Debug, Clone)]
pub struct Local {}

impl Local {
	/// Instantiates the local setup strategy with ports on localhost
	pub fn new() -> Self {
		Self {}
	}
}

impl Default for Local {
	fn default() -> Self {
		Local::new()
	}
}

impl Local {
        pub async fn setup(
                &self, 
                dot_movement: &DotMovement,
		mut config: BridgeConfig,
        ) -> Result<(BridgeConfig, AnvilInstance, Child), anyhow::Error> {
                let eth_config = EthConfig::build_for_test();
                let eth_client = EthClient::new(eth_config).await?;
                let anvil = Anvil::new().port(eth_client.rpc_port()).spawn();
                    
                let (_movement_client, child) = MovementClient::new_for_test(MovementConfig::build_for_test()).await?; 
        
                Ok((config, anvil, child))
        }

}