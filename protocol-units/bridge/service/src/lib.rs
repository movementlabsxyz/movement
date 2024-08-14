use bridge_shared::blockchain_service::AbstractBlockchainService;
use ethereum_bridge::{client::EthClient, event_logging::EthInitiatorMonitoring};

pub type EthereumService = AbstractBlockchainService<
EthClient, EthInitiatorMonitoring<EthAddress, EthHash>>
