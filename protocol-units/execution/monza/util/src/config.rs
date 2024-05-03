use aptos_sdk::types::chain_id::ChainId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub chain_id : ChainId,
    pub aptos_rest_listen_addr : String,
    pub aptos_port_listen_addr : String,
    pub light_node_config : m1_da_light_node_util::Config,
}

impl Config {

    pub const CHAIN_ID_ENV_VAR : &'static str = "MONZA_CHAIN_ID";
    pub const APTOS_REST_LISTEN_ADDR_ENV_VAR : &'static str = "MONZA_APTOS_REST_LISTEN_ADDR";
    pub const APTOS_PORT_LISTEN_ADDR_ENV_VAR : &'static str = "MONZA_APTOS_PORT_LISTEN_ADDR";

    pub fn new(chain_id : ChainId, aptos_rest_listen_addr : String, aptos_port_listen_addr : String, light_node_config : m1_da_light_node_util::Config) -> Self {
        Self {
            chain_id,
            aptos_rest_listen_addr,
            aptos_port_listen_addr,
            light_node_config,
        }
    }

    pub fn try_from_env() -> Result<Self, anyhow::Error> {

        let chain_id = match std::env::var(Self::CHAIN_ID_ENV_VAR) {
            Ok(chain_id) => ChainId::new(chain_id.parse()?),
            Err(_) => ChainId::default()
        };

        let aptos_rest_listen_addr = std::env::var(Self::APTOS_REST_LISTEN_ADDR_ENV_VAR)
        .unwrap_or("0.0.0.0:30731".to_string());

        let aptos_port_listen_addr = std::env::var(Self::APTOS_PORT_LISTEN_ADDR_ENV_VAR)
        .unwrap_or("0.0.0.0:30732".to_string());

        let light_node_config = m1_da_light_node_util::Config::try_from_env()?;

        Ok(Self {
            chain_id,
            aptos_rest_listen_addr,
            aptos_port_listen_addr,
            light_node_config,
        })
        
    }

}