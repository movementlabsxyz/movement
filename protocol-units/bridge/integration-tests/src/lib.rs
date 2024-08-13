use alloy::{
	node_bindings::Anvil,
	primitives::Address,
	providers::WalletProvider,
	signers::{
		k256::{elliptic_curve::SecretKey, Secp256k1},
		local::LocalSigner,
	},
};
use alloy_network::{Ethereum, EthereumWallet, NetworkWallet};
use anyhow::Result;
use aptos_sdk::types::LocalAccount;
use ethereum_bridge::{
	types::{AlloyProvider, AtomicBridgeInitiator, EthAddress},
	Config as EthConfig, EthClient,
};
use movement_bridge::{MovementClient, Config as MovementConfig};
use rand::SeedableRng;
use aptos_logger::Logger;
use aptos_language_e2e_tests::{
	account::Account, common_transactions::peer_to_peer_txn, executor::FakeExecutor,
};
use aptos_types::{
	account_config::{DepositEvent, WithdrawEvent},
	transaction::{ExecutionStatus, SignedTransaction, TransactionOutput, TransactionStatus},
};
use std::{convert::TryFrom, time::Instant};

alloy::sol!(
	#[allow(missing_docs)]
	#[sol(rpc)]
	WETH9,
	"../chains/ethereum/abis/WETH9.json"
);
pub struct TestHarness {
	pub eth_client: Option<EthClient>,
	pub movement_client: Option<MovementClient>,
}

impl TestHarness {

	pub async fn new_with_movement() -> Self {
		std::env::set_var("DOT_MOVEMENT_PATH", ".");
		let movement_client = MovementClient::new_for_test(MovementConfig::build_for_test())
			.await
			.expect("Failed to create EthClient");
		Self { eth_client: None, movement_client: Some(movement_client) }
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
		Self { eth_client: Some(eth_client), movement_client: None }
	}

	pub fn eth_client(&self) -> Result<&EthClient> {
		self.eth_client.as_ref().ok_or(anyhow::Error::msg("EthClient not initialized"))
	}

	pub fn eth_client_mut(&mut self) -> Result<&mut EthClient> {
		self.eth_client.as_mut().ok_or(anyhow::Error::msg("EthClient not initialized"))
	}

	pub fn set_eth_signer(&mut self, signer: SecretKey<Secp256k1>) -> Address {
		let eth_client = self.eth_client_mut().expect("EthClient not initialized");
		let wallet: &mut EthereumWallet = eth_client.rpc_provider_mut().wallet_mut();
		wallet.register_default_signer(LocalSigner::from(signer));
		<EthereumWallet as NetworkWallet<Ethereum>>::default_signer_address(wallet)
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
		let eth_client = self.eth_client_mut().expect("EthClient not initialized");
		let contract = AtomicBridgeInitiator::deploy(eth_client.rpc_provider())
			.await
			.expect("Failed to deploy AtomicBridgeInitiator");
		eth_client.set_initiator_contract(contract.with_cloned_provider());
		eth_client.initiator_contract_address().expect("Initiator contract not set")
	}

	pub async fn deploy_weth_contract(&mut self) -> Address {
		let eth_client = self.eth_client_mut().expect("EthClient not initialized");
		let weth = WETH9::deploy(eth_client.rpc_provider()).await.expect("Failed to deploy WETH9");
		weth.address().to_owned()
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

	pub fn gen_aptos_account(&self) -> Vec<u8> {
		let mut rng = ::rand::rngs::StdRng::from_seed([3u8; 32]);
		let movement_recipient = LocalAccount::generate(&mut rng);
		movement_recipient.public_key().to_bytes().to_vec()
	}
}
