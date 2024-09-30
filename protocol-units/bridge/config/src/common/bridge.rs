use alloy::{providers::RootProvider, pubsub::PubSubFrontend};
use godfig::env_default;
use serde::{Deserialize, Serialize};
use aptos_sdk::{
	coin_client::{CoinClient, TransferOptions},
	move_types::{identifier::Identifier, language_storage::ModuleId},
	rest_client::{Client, FaucetClient},
	transaction_builder::TransactionBuilder,
	types::{chain_id::ChainId, transaction::EntryFunction, LocalAccount},
};
use bridge_service::chains::ethereum::types::{
	AlloyProvider, AtomicBridgeInitiator, EthAddress, EthConfig,
};
use rand::rngs::OsRng;
use std::sync::Arc;

fn init_default_movement_signer() -> Arc<LocalAccount> {
	let mut rng = OsRng;
	Arc::new(LocalAccount::generate(&mut rng))
}

const DEFAULT_MOVEMENT_NATIVE_ADDRESS: &str = "0xface";
const DEFAULT_MOVEMENT_NON_NATIVE_ADDRESS: &str = "0xdafe";
const DEFAULT_MOVEMENT_REST_CLIENT: &str = "https://aptos.devnet.suzuka.movementlabs.xyz/v1";
const DEFAULT_MOVEMENT_FAUCET_CLIENT: &str = "https://faucet.devnet.suzuka.movementlabs.xyz/";
const DEFAULT_MOVEMENT_SIGNER: Option<Arc<LocalAccount>> = None;
const DEFAULT_ETH_RPC_PROVIDER: AlloyProvider = AlloyProvider::new;
const DEFAULT_ETH_RPC_PORT: &str = "8545";
const DEFAULT_ETH_WS_PROVIDER: Option<RootProvider<PubSubFrontend>> = None;
const DEFAULT_ETH_INITIATOR_CONTRACT: &str = "Oxeee";
const DEFAULT_ETH_COUNTERPARTY_CONTRACT: &str = "0xccc";
const DEFAULT_ETH_WETH_CONTRACT: &str = "0xe3e3";
const DEFAULT_ETH_CONFIG: EthConfig = EthConfig::default;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
        // Movement config 
        #[serde(default = "default_movement_native_address")]
        pub movement_native_address: AccountAddress,
        #[serde(default = "default_movement_non_native_address")]
	pub movement_non_native_address: Vec<u8>,
        #[serde(default = "default_movement_rest_client")]
	pub movement_rest_client: Client,
        #[serde(default = "default_movement_faucet_client")]
	pub movement_faucet_client: Option<Arc<RwLock<FaucetClient>>>,
        #[serde(default = "default_movement_signer")]
	movement_signer: Arc<LocalAccount>,

        // Eth config
        #[serde(default = "default_eth_rpc_provider")]
        eth_rpc_provider: AlloyProvider,
        #[serde(default = "default_eth_rpc_port")]
	eth_rpc_port: u16,
        #[serde(default = "default_eth_ws_provider")]
	eth_ws_provider: Option<RootProvider<PubSubFrontend>>,
        #[serde(default = "default_eth_initiator_contract")]
	eth_initiator_contract: InitiatorContract,
        #[serde(default = "default_eth_counterparty_contract")]
	eth_counterparty_contract: CounterpartyContract,
        #[serde(default = "default_eth_weth_contract")]
	eth_weth_contract: WETH9Contract,
        #[serde(default = "default_eth_config")]
	eth_config: Config,
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
	default_movement_signer,
	"MOVEMENT_SIGNER",
	String,
	DEFAULT_MOVEMENT_SIGNER.to_string()
);

env_default!(
	default_eth_rpc_provider,
	"ETH_RPC_PROVIDER",
	String,
	DEFAULT_ETH_RPC_PROVIDER.to_string()
);

env_default!(
	default_eth_rpc_port,
	"ETH_RPC_PORT",
	String,
	DEFAULT_ETH_RPC_PORT.to_string()
);

env_default!(
	default_eth_ws_provider,
	"ETH_WS_PROVIDER",
	String,
	DEFAULT_ETH_WS_PROVIDER.to_string()
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
	DEFAULT_ETH_WS_CONNECTION_PORT
);

env_default!(
	default_eth_weth_contract,
	"ETH_WETH_CONTRACT",
	String,
	DEFAULT_ETH_WS_CONNECTION_PORT
);

env_default!(
	default_eth_config,
	"ETH_CONFIG",
	String,
	DEFAULT_ETH_WS_CONNECTION_PORT
);

impl Default for Config {
	fn default() -> Self {
                Config {
                        movement_native_address: default_movement_native_address(),
                        movement_non_native_address: default_movement_non_native_address(),
                        movement_rest_client: default_movement_rest_client(),
                        movement_faucet_client: default_movement_rest_client(),
                        movement_signer: default_movement_signer(),
                        eth_rpc_provider: default_eth_rpc_provider(),
                        eth_rpc_port: default_eth_rpc_port(),
                        eth_ws_provider: default_eth_ws_provider(),
                        eth_initiator_contract: default_eth_initiator_contract(),
                        eth_counterparty_contract: default_eth_counterparty_contract(),
                        eth_weth_contract: default_eth_weth_contract(),
                        eth_config: default_eth_config(),
		}
	}
}

impl Config {
	pub fn eth_rpc_connection_url(&self) -> String {
		format!(
			"{}://{}:{}",
			self.eth_rpc_connection_protocol,
			self.eth_rpc_connection_hostname,
			self.eth_rpc_connection_port
		)
	}

	pub fn eth_ws_connection_url(&self) -> String {
		format!(
			"{}://{}:{}",
			self.eth_ws_connection_protocol,
			self.eth_ws_connection_hostname,
			self.eth_ws_connection_port
		)
	}
}
