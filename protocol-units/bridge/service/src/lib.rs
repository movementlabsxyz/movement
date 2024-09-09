use bridge_shared::{
	blockchain_service::AbstractBlockchainService,
	bridge_service::{BridgeService, BridgeServiceConfig},
};
use ethereum_bridge::{
	client::{Config, EthClient},
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

pub fn setup_bridge_service(config: BridgeServiceConfig) -> SetupBridgeServiceResult {
	let mut rng = TestRng::new([0u8; 32]);
	let mut ethereum_service = EthereumChain::new(rng.clone(), "Ethereum");
	let mut movement_service = MovementChain::new(rng.clone(), "Movement");

	//@TODO: use json config instead of build_for_test
	let config = Config::build_for_test();

	let eth_client = EthClient::new(config);
	let temp_rpc_url = "http://localhost:8545";
	let eth_initiator_monitoring =
		EthInitiatorMonitoring::build(temp_rpc_url, ethereum_service.add_event_listener());
}
