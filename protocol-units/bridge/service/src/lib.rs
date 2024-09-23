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
	utils::{MovementAddress, MovementHash},
	MovementChain,
};
use rand::SeedableRng;

pub type EthereumService = AbstractBlockchainService<
	EthClient,
	EthInitiatorMonitoring<EthAddress, EthHash>,
	EthClient,
	EthCounterpartyMonitoring<EthAddress, EthHash>,
	EthAddress,
	EthHash,
>;

pub type MovementService = AbstractBlockchainService<
	MovementClient,
	MovementInitiatorMonitoring<MovementAddress, MovementHash>,
	MovementClient,
	MovementCounterpartyMonitoring<MovementAddress, MovementHash>,
	MovementAddress,
	MovementHash,
>;

pub struct SetupBridgeService(
	pub BridgeService<EthereumService, MovementService>,
	pub EthClient,
	pub MovementClient,
	pub EthereumChain,
	pub MovementChain,
);

pub async fn setup_bridge_service(bridge_config: BridgeServiceConfig) -> SetupBridgeService {
	let mut rng = TestRng::from_seed([0u8; 32]);
	let mut ethereum_service = EthereumChain::new("Ethereum".to_string(), "localhost:8545").await;
	let mut movement_service = MovementChain::new();

	//@TODO: use json config instead of build_for_test
	let config = EthConfig::build_for_test();

	let eth_client = EthClient::new(config).await.expect("Faile to creaet EthClient");
	let temp_rpc_url = "http://localhost:8545";
	let eth_initiator_monitoring = EthInitiatorMonitoring::build(temp_rpc_url.clone())
		.await
		.expect("Failed to create EthInitiatorMonitoring");
	let eth_conterparty_monitoring = EthCounterpartyMonitoring::build(temp_rpc_url)
		.await
		.expect("Failed to create EthCounterpartyMonitoring");

	let movement_counterparty_monitoring = MovementCounterpartyMonitoring::build("localhost:8080")
		.await
		.expect("Failed to create MovementCounterpartyMonitoring");
	let movement_initiator_monitoring = MovementInitiatorMonitoring::build("localhost:8080")
		.await
		.expect("Failed to create MovementInitiatorMonitoring");

	//@TODO: use json config instead of build_for_test
	let config = MovementConfig::build_for_test();

	let ethereum_chain = EthereumService {
		initiator_contract: eth_client.clone(),
		initiator_monitoring: eth_initiator_monitoring,
		counterparty_contract: eth_client.clone(),
		counterparty_monitoring: eth_conterparty_monitoring,
		_phantom: Default::default(),
	};

	let movement_client =
		MovementClient::new(config).await.expect("Failed to create MovementClient");

	let movement_chain = MovementService {
		initiator_contract: movement_client.clone(),
		initiator_monitoring: movement_initiator_monitoring,
		counterparty_contract: movement_client.clone(),
		counterparty_monitoring: movement_counterparty_monitoring,
		_phantom: Default::default(),
	};

	// EthereumChain must be BlockchainService
	let bridge_service = BridgeService::new(ethereum_chain, movement_chain, bridge_config);

	SetupBridgeService(
		bridge_service,
		eth_client,
		movement_client,
		ethereum_service,
		movement_service,
	)
}
