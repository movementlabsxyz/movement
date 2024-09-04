use alloy::{
	primitives::{Address, U256},
	providers::WalletProvider,
	signers::{
		k256::{elliptic_curve::SecretKey, Secp256k1},
		local::LocalSigner,
	},
};
use alloy_network::{Ethereum, EthereumWallet, NetworkWallet};
use anyhow::Result;
use aptos_sdk::rest_client::{Client, FaucetClient};
use aptos_sdk::types::LocalAccount;
use bridge_shared::bridge_contracts::{BridgeContractInitiator, BridgeContractInitiatorResult};
use bridge_shared::types::{Amount, HashLock, InitiatorAddress, RecipientAddress, TimeLock};
use ethereum_bridge::client::{Config as EthConfig, EthClient};
use ethereum_bridge::types::{AlloyProvider, AtomicBridgeInitiator, EthAddress, WETH9};
use movement_bridge::Config as MovementConfig;
use movement_bridge::MovementClient;
use rand::SeedableRng;
use std::{
	env,
	net::TcpStream,
	sync::{Arc, RwLock},
	time::Duration,
};
use tokio::process::Command;

pub struct TestHarness {
	pub eth_client: Option<EthClient>,
	pub movement_client: Option<MovementClient>,
	pub indexer_process: Option<tokio::process::Child>,
}

impl TestHarness {
	pub async fn new_with_movement() -> (Self, tokio::process::Child) {
		let (movement_client, child) =
			MovementClient::new_for_test(MovementConfig::build_for_test())
				.await
				.expect("Failed to create MovementClient");
		(
			Self {
				eth_client: None,
				movement_client: Some(movement_client),
				indexer_process: None,
			},
			child,
		)
	}

	pub fn movement_rest_client(&self) -> &Client {
		self.movement_client().expect("Could not fetch Movement client").rest_client()
	}

	pub fn movement_faucet_client(&self) -> &Arc<RwLock<FaucetClient>> {
		self.movement_client()
			.expect("Could not fetch Movement client")
			.faucet_client()
			.expect("Faucet client not initialized")
	}

	pub fn movement_client(&self) -> Result<&MovementClient> {
		self.movement_client
			.as_ref()
			.ok_or(anyhow::Error::msg("MovementClient not initialized"))
	}

	pub fn movement_client_mut(&mut self) -> Result<&mut MovementClient> {
		self.movement_client
			.as_mut()
			.ok_or(anyhow::Error::msg("MovementClient not initialized"))
	}

	pub async fn new_only_eth() -> Self {
		let eth_client = EthClient::new(EthConfig::build_for_test())
			.await
			.expect("Failed to create EthClient");
		Self { eth_client: Some(eth_client), movement_client: None, indexer_process: None }
	}

	pub async fn new_only_indexer() {}

	pub fn eth_client(&self) -> Result<&EthClient> {
		self.eth_client.as_ref().ok_or(anyhow::Error::msg("EthClient not initialized"))
	}

	pub fn eth_client_mut(&mut self) -> Result<&mut EthClient> {
		self.eth_client.as_mut().ok_or(anyhow::Error::msg("EthClient not initialized"))
	}

	pub fn set_eth_signer(&mut self, signer: SecretKey<Secp256k1>) -> Address {
		let eth_client = self.eth_client_mut().expect("EthClient not initialized");
		let wallet: &mut EthereumWallet = eth_client.rpc_provider_mut().wallet_mut();
		let clone_signer = signer.clone();
		wallet.register_default_signer(LocalSigner::from(signer));
		eth_client.set_signer_address(clone_signer);
		eth_client.get_signer_address()
	}

	pub fn eth_signer_address(&self) -> Address {
		let eth_client = self.eth_client().expect("EthClient not initialized");
		let wallet: &EthereumWallet = eth_client.rpc_provider().wallet();
		<EthereumWallet as NetworkWallet<Ethereum>>::default_signer_address(wallet)
	}

	pub fn provider(&self) -> &AlloyProvider {
		self.eth_client().expect("Could not fetch eth client").rpc_provider()
	}

	/// The port that Anvil will listen on.
	pub fn rpc_port(&self) -> u16 {
		self.eth_client().expect("Could not fetch eth client").rpc_port()
	}

	pub async fn deploy_initiator_contract(&mut self) -> Address {
		let eth_client: &mut EthClient = self.eth_client_mut().expect("EthClient not initialized");
		let contract = AtomicBridgeInitiator::deploy(eth_client.rpc_provider())
			.await
			.expect("Failed to deploy AtomicBridgeInitiator");
		eth_client.set_initiator_contract(contract.with_cloned_provider());
		eth_client.initiator_contract_address().expect("Initiator contract not set")
	}

	pub async fn deploy_weth_contract(&mut self) -> Address {
		let eth_client = self.eth_client_mut().expect("EthClient not initialized");
		let weth = WETH9::deploy(eth_client.rpc_provider()).await.expect("Failed to deploy WETH9");
		eth_client.set_weth_contract(weth.with_cloned_provider());
		eth_client.weth_contract_address().expect("WETH contract not set")
	}

	pub async fn deploy_init_contracts(&mut self) {
		let _ = self.deploy_initiator_contract().await;
		let weth_address = self.deploy_weth_contract().await;
		self.eth_client()
			.expect("Failed to get EthClient")
			.initialize_initiator_contract(
				EthAddress(weth_address),
				EthAddress(self.eth_signer_address()),
			)
			.await
			.expect("Failed to initialize contract");
	}

	pub async fn initiate_bridge_transfer(
		&mut self,
		initiator_address: InitiatorAddress<EthAddress>,
		recipient_address: RecipientAddress<Vec<u8>>,
		hash_lock: HashLock<[u8; 32]>,
		time_lock: TimeLock,
		amount: Amount, // the amount
	) -> BridgeContractInitiatorResult<()> {
		let eth_client = self.eth_client_mut().expect("EthClient not initialized");
		eth_client
			.initiate_bridge_transfer(
				initiator_address,
				recipient_address,
				hash_lock,
				time_lock,
				amount,
			)
			.await
	}

	pub async fn deposit_weth_and_approve(
		&mut self,
		initiator_address: InitiatorAddress<EthAddress>,
		amount: Amount,
	) -> BridgeContractInitiatorResult<()> {
		let eth_client = self.eth_client_mut().expect("EthClient not initialized");
		eth_client
			.deposit_weth_and_approve(initiator_address.0 .0, U256::from(amount.weth()))
			.await
			.expect("Failed to deposit WETH");
		Ok(())
	}

	pub fn gen_aptos_account(&self) -> Vec<u8> {
		let mut rng = ::rand::rngs::StdRng::from_seed([3u8; 32]);
		let movement_recipient = LocalAccount::generate(&mut rng);
		movement_recipient.public_key().to_bytes().to_vec()
	}

	pub async fn start_indexer(&self, host: &str, port: u16) -> tokio::process::Child {
		//check that the harness is initialized with a movement client
		let _ = self
			.movement_client()
			.expect("Movement Client and Harness must be initialized to start the indexer");

		let package_root = env::var("CARGO_MANIFEST_DIR").unwrap();
		let script_path = format!("{}/scripts/postgres-start.bash", package_root);

		// Check if PostgreSQL is running
		if !is_postgres_running(host, port).await {
			println!("PostgreSQL is not running. Starting PostgreSQL setup...");

			tokio::spawn(async move {
				Command::new("bash")
					.arg(&script_path)
					.status()
					.await
					.expect("Failed to run the PostgreSQL setup script")
			});

			tokio::time::sleep(Duration::from_secs(5)).await;
		} else {
			println!("PostgreSQL is already running.");
		}

		let indexer_child = Command::new("cargo")
			.arg("run")
			.arg("-p")
			.arg("suzuka-indexer-service")
			.env("MAPTOS_CHAIN_ID", "4")
			.env("MAPTOS_INDEXER_GRPC_LISTEN_HOSTNAME", "127.0.0.1")
			.env("MAPTOS_INDEXER_GRPC_LISTEN_PORT", "50051")
			.env("DOT_MOVEMENT_PATH", format!("{}/.movement", package_root))
			.spawn()
			.expect("Failed to start indexer");

		indexer_child
	}
}

async fn is_postgres_running(host: &str, port: u16) -> bool {
	TcpStream::connect((host, port)).is_ok()
}
