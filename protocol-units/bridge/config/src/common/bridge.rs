use godfig::env_default;
use serde::{Deserialize, Serialize};

const DEFAULT_MOVEMENT_NATIVE_ADDRESS: &str = "0xface";
const DEFAULT_MOVEMENT_NON_NATIVE_ADDRESS: &str = "0xdafe";
const DEFAULT_MOVEMENT_REST_CLIENT: &str = "https://aptos.devnet.suzuka.movementlabs.xyz/v1";
const DEFAULT_MOVEMENT_FAUCET_CLIENT: &str = "https://faucet.devnet.suzuka.movementlabs.xyz/";
const DEFAULT_ETH_RPC_PORT: &str = "8545";
const DEFAULT_ETH_WS_CONTRACT: &str = "0xe3e3";
const DEFAULT_ETH_INITIATOR_CONTRACT: &str = "Oxeee";
const DEFAULT_ETH_COUNTERPARTY_CONTRACT: &str = "0xccc";
const DEFAULT_ETH_WETH_CONTRACT: &str = "0xe3e3";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
        // Movement config 
        #[serde(default = "default_movement_native_address")]
        pub movement_native_address: String,
        #[serde(default = "default_movement_non_native_address")]
	pub movement_non_native_address: String,
        #[serde(default = "default_movement_rest_client")]
	pub movement_rest_client: String,
        #[serde(default = "default_movement_faucet_client")]
	pub movement_faucet_client: String,

        // Eth config
        #[serde(default = "default_eth_rpc_port")]
	eth_rpc_port: String,
        #[serde(default = "default_eth_initiator_contract")]
	eth_initiator_contract: String,
        #[serde(default = "default_eth_counterparty_contract")]
	eth_counterparty_contract: String,
        #[serde(default = "default_eth_weth_contract")]
	eth_weth_contract: String,
}

env_default!(
	default_movement_native_address,
	"MOVEMENT_NATIVE_ADDRESS",
	String,
	DEFAULT_MOVEMENT_NATIVE_ADDRESS.to_string()
);

env_default!(
	default_movement_non_native_address,
	"MOVEMENT_NON_NATIVE_ADDRESS",
	String,
	DEFAULT_MOVEMENT_NON_NATIVE_ADDRESS.to_string()
);

env_default!(
	default_movement_rest_client,
	"MOVEMENT_REST_CLIENT",
	String,
	DEFAULT_MOVEMENT_REST_CLIENT.to_string()
);

env_default!(
	default_movement_faucet_client,
	"MOVEMENT_FAUCET_CLIENT",
	String,
	DEFAULT_MOVEMENT_FAUCET_CLIENT.to_string()
);


env_default!(
	default_eth_rpc_port,
	"ETH_RPC_PORT",
	String,
	DEFAULT_ETH_RPC_PORT.to_string()
);

env_default!(
	default_eth_initiator_contract,
	"ETH_INITIATOR_CONTRACT",
	String,
	DEFAULT_ETH_INITIATOR_CONTRACT.to_string()
);

env_default!(
	default_eth_counterparty_contract,
	"ETH_COUNTERPARTY_CONTRACT",
	String,
	DEFAULT_ETH_COUNTERPARTY_CONTRACT.to_string()
);

env_default!(
	default_eth_weth_contract,
	"ETH_WETH_CONTRACT",
	String,
	DEFAULT_ETH_WETH_CONTRACT.to_string()
);

impl Default for Config {
	fn default() -> Self {
                Config {
                        movement_native_address: default_movement_native_address(),
                        movement_non_native_address: default_movement_non_native_address(),
                        movement_rest_client: default_movement_rest_client(),
                        movement_faucet_client: default_movement_rest_client(),
                        eth_rpc_port: default_eth_rpc_port(),
                        eth_initiator_contract: default_eth_initiator_contract(),
                        eth_counterparty_contract: default_eth_counterparty_contract(),
                        eth_weth_contract: default_eth_weth_contract(),
		}
	}
}
