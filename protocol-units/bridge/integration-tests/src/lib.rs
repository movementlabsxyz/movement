#![allow(dead_code)]
use alloy::node_bindings::AnvilInstance;
use alloy::primitives::Address;
use alloy::primitives::{keccak256, U256};
use alloy::providers::ProviderBuilder;
use alloy::signers::local::PrivateKeySigner;
use alloy_network::EthereumWallet;
use aptos_sdk::rest_client::{Client, FaucetClient};
use aptos_sdk::types::account_address::AccountAddress;
use aptos_sdk::types::LocalAccount;
use bridge_config::Config;
use bridge_service::chains::ethereum::types::AlloyProvider;
use bridge_service::chains::ethereum::types::EthAddress;
use bridge_service::chains::ethereum::{client::EthClient, types::EthHash};
use bridge_service::chains::movement::utils::MovementAddress;
use bridge_service::chains::movement::{client::MovementClient, utils::MovementHash};
use bridge_service::types::Amount;
use bridge_service::{chains::bridge_contracts::BridgeContractResult, types::BridgeAddress};
use rand::SeedableRng;
use tokio::process::Command;
use std::str::FromStr;
use std::sync::{Arc, RwLock};
use url::Url;

pub mod utils;

#[derive(Clone)]
pub struct EthToMovementCallArgs {
	pub initiator: Vec<u8>,
	pub recipient: MovementAddress,
	pub bridge_transfer_id: MovementHash,
	pub hash_lock: MovementHash,
	pub time_lock: u64,
	pub amount: u64,
}

#[derive(Clone)]
pub struct MovementToEthCallArgs {
	pub initiator: MovementAddress,
	pub recipient: Vec<u8>,
	pub bridge_transfer_id: EthHash,
	pub hash_lock: EthHash,
	pub time_lock: u64,
	pub amount: u64,
}

impl Default for EthToMovementCallArgs {
	fn default() -> Self {
		Self {
			initiator: b"0x123".to_vec(),
			recipient: MovementAddress(AccountAddress::new(*b"0x00000000000000000000000000face")),
			bridge_transfer_id: MovementHash(*b"00000000000000000000000transfer1"),
			hash_lock: MovementHash(*keccak256(b"secret")),
			time_lock: 3600,
			amount: 100,
		}
	}
}

impl Default for MovementToEthCallArgs {
	fn default() -> Self {
		let preimage = "secret".to_string();
		let serialized_preimage = bcs::to_bytes(&preimage).unwrap();
		let hash_lock = *keccak256(&serialized_preimage);

		Self {
			initiator: MovementAddress(AccountAddress::new(*b"0x00000000000000000000000000face")),
			recipient: b"0x123".to_vec(),
			bridge_transfer_id: EthHash(*b"00000000000000000000000transfer1"),
			hash_lock: EthHash(hash_lock),
			time_lock: 3600,
			amount: 100,
		}
	}
}

pub struct HarnessEthClient {
	pub eth_rpc_url: String,
	pub signer_private_key: PrivateKeySigner,
	pub eth_client: EthClient,
	pub anvil: AnvilInstance,
}

impl HarnessEthClient {
	pub async fn rpc_provider(&self) -> AlloyProvider {
		let rpc_provider = ProviderBuilder::new()
			.with_recommended_fillers()
			.wallet(EthereumWallet::from(self.signer_private_key.clone()))
			.on_builtin(&self.eth_rpc_url)
			.await
			.expect("Error during provider creation");
		rpc_provider
	}

	pub fn signer_address(&self) -> Address {
		self.signer_private_key.address()
	}

	pub fn get_initiator_private_key(&self) -> PrivateKeySigner {
		self.anvil.keys()[2].clone().into()
	}

	pub fn get_initiator_address(&self) -> Address {
		self.get_initiator_private_key().address()
	}

	pub fn get_recipient_private_key(&self) -> PrivateKeySigner {
		self.anvil.keys()[3].clone().into()
	}

	pub fn get_recipeint_address(&self) -> Address {
		self.get_recipient_private_key().address()
	}

	pub async fn deposit_weth_and_approve(
		&mut self,
		initiator_address: BridgeAddress<EthAddress>,
		amount: Amount,
	) -> BridgeContractResult<()> {
		self.eth_client
			.deposit_weth_and_approve(initiator_address.0 .0, U256::from(amount.weth_value()))
			.await
			.expect("Failed to deposit WETH");
		Ok(())
	}
}

pub struct HarnessMvtClient {
	pub movement_client: MovementClient,
	pub movement_process: tokio::process::Child,
	///The Apotos Rest Client
	pub rest_client: Client,
	///The Apotos Rest Client
	pub faucet_client: Arc<RwLock<FaucetClient>>,
}

impl HarnessMvtClient {
	pub fn gen_aptos_account() -> Vec<u8> {
		let mut rng = ::rand::rngs::StdRng::from_seed([3u8; 32]);
		let movement_recipient = LocalAccount::generate(&mut rng);
		movement_recipient.public_key().to_bytes().to_vec()
	}
}

pub struct TestHarness;
impl TestHarness {
	pub async fn new_with_eth_and_movement(
		config: Config,
	) -> (HarnessEthClient, HarnessMvtClient, Config) {
		let (eth_client_harness, config) = TestHarness::new_only_eth(config).await;
		let (mvt_client_harness, config) = TestHarness::new_with_movement(config).await;
		(eth_client_harness, mvt_client_harness, config)
	}

	pub async fn new_with_movement(config: Config) -> (HarnessMvtClient, Config) {
		let (config, movement_process) = bridge_setup::test_mvt_setup(config)
			.await
			.expect("Failed to setup Movement config");

		let movement_client = MovementClient::new(&config.movement)
			.await
			.expect("Failed to create MovementClient");

		let node_connection_url = Url::from_str(&config.movement.mvt_rpc_connection_url())
			.expect("Bad movement rpc url in config");
		let rest_client = Client::new(node_connection_url.clone());

		let faucet_url = Url::from_str(&config.movement.mvt_faucet_connection_url())
			.expect("Bad movement faucet url in config");
		let faucet_client = Arc::new(RwLock::new(FaucetClient::new(
			faucet_url.clone(),
			node_connection_url.clone(),
		)));

		(HarnessMvtClient { movement_client, movement_process, rest_client, faucet_client }, config)
	}

	pub async fn new_with_suzuka(config: Config) -> (HarnessMvtClient, Config) {
		let movement_client = MovementClient::new(&config.movement)
			.await
			.expect("Failed to create MovementClient");

		let node_connection_url = Url::from_str(&config.movement.mvt_rpc_connection_url())
			.expect("Bad movement rpc url in config");
		let rest_client = Client::new(node_connection_url.clone());

		let faucet_url = Url::from_str(&config.movement.mvt_faucet_connection_url())
			.expect("Bad movement faucet url in config");
		let faucet_client = Arc::new(RwLock::new(FaucetClient::new(
			faucet_url.clone(),
			node_connection_url.clone(),
		)));

		let movement_process = Command::new("sleep")
		.arg("1000")
		.spawn()
		.expect("failed to spawn dummy process");

		(HarnessMvtClient { movement_client, movement_process, rest_client, faucet_client }, config)
	}

	pub async fn new_only_eth(config: Config) -> (HarnessEthClient, Config) {
		let (config, anvil) = bridge_setup::test_eth_setup(config)
			.await
			.expect("Test eth config setup failed.");

		let eth_rpc_url = config.eth.eth_rpc_connection_url().clone();

		let signer_private_key = config
			.eth
			.signer_private_key
			.parse::<PrivateKeySigner>()
			.expect("Error during parsing signer private key?");

		let eth_client = EthClient::new(&config.eth).await.expect("Failed to create EthClient");
		(HarnessEthClient { eth_client, eth_rpc_url, signer_private_key, anvil }, config)
	}
}
