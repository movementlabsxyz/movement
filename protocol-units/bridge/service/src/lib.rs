use bridge_shared::{
	blockchain_service::AbstractBlockchainService,
	bridge_service::{BridgeService, BridgeServiceConfig},
};
use ethereum_bridge::{
	client::{Config as EthConfig, EthClient},
	event_monitoring::{EthCounterpartyMonitoring, EthInitiatorMonitoring},
	types::{EthAddress, EthHash},
	utils::TestRng,
	EthereumChain,
};
use movement_bridge::{
	client::{Config as MovementConfig, MovementClient},
	event_monitoring::{MovementCounterpartyMonitoring, MovementInitiatorMonitoring},
	types::MovementHash,
	utils::MovementAddress,
	MovementChain,
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
	let config = EthConfig::build_for_test();

	let eth_client = EthClient::new(config);
	let temp_rpc_url = "http://localhost:8545";
	let eth_initiator_monitoring =
		EthInitiatorMonitoring::build(temp_rpc_url.clone(), ethereum_service.add_event_listener());
	let eth_conterparty_monitoring =
		EthCounterpartyMonitoring::build(temp_rpc_url, ethereum_service.add_event_listener());

	let movement_counterparty_monitoring = MovementCounterpartyMonitoring::build(
		"localhost:8080",
		movement_service.add_event_listener(),
	);

	let movement_initiator_monitoring =
		MovementInitiatorMonitoring::build("localhost:8080", movement_service.add_event_listener());

	//@TODO: use json config instead of build_for_test
	let config = Config::build_for_test();
	let movement_client = MovementClient::new(config);

	let movement_client = MovementClient::new(config);
	let eth_service = EthereumService {
		initiator_contract: eth_client.clone(),
		initiator_monitoring: eth_initiator_monitoring,
		counterparty_contract: eth_client.clone(),
		counterparty_monitoring: eth_conterparty_monitoring,
		_phantom: Default::default(),
	};
}
