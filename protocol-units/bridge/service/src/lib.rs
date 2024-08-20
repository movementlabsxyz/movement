use bridge_shared::{blockchain_service::AbstractBlockchainService, bridge_service::BridgeService};
use ethereum_bridge::{
	client::EthClient,
	event_monitoring::EthInitiatorMonitoring,
	types::{EthAddress, EthHash},
	utils::TestRng,
	EthereumChain,
};
use movement_bridge::{
	client::MovementClient, event_monitoring::MovementCounterpartyMonitoring, types::MovementHash,
	utils::MovementAddress, MovementChain,
};

pub type EthereumService = AbstractBlockchainService<
	EthClient,
	EthInitiatorMonitoring<EthAddress, EthHash>,
	MovementClient,
	MovementCounterpartyMonitoring<MovementAddress, MovementHash>,
	EthAddress,
	EthHash,
>;

pub type MovementService = AbstractBlockchainService<
	MovementClient,
	MovementCounterpartyMonitoring<MovementAddress, MovementHash>,
	EthClient,
	EthInitiatorMonitoring<EthAddress, EthHash>,
	MovementAddress,
	MovementHash,
>;

pub struct SetupBridgeServiceResult(
	pub BridgeService<EthereumService, MovementService>,
	pub EthClient,
	pub MovementClient,
	pub EthereumChain<EthAddress, EthHash, TestRng>,
	pub MovementChain<MovementAddress, MovementHash, TestRng>,
);
