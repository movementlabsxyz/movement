use super::utils::{calculate_storage_slot, send_transaction, send_transaction_rules};
use crate::chains::bridge_contracts::BridgeContractError;
use crate::chains::bridge_contracts::BridgeContractResult;
use alloy::primitives::{private::serde::Deserialize, Address, FixedBytes, U256};
use alloy::providers::{Provider, ProviderBuilder, RootProvider};
use alloy::{
	network::EthereumWallet,
	rlp::{RlpDecodable, RlpEncodable},
};
use alloy::{pubsub::PubSubFrontend, signers::local::PrivateKeySigner};
use alloy_rlp::Decodable;
use serde_with::serde_as;
use std::fmt::{self, Debug};
use url::Url;

use crate::types::{
	Amount, AssetType, BridgeAddress, BridgeTransferDetails, BridgeTransferId, HashLock,
	HashLockPreImage, TimeLock,
};

use super::types::{
	AlloyProvider, AtomicBridgeCounterparty, AtomicBridgeInitiator, CounterpartyContract,
	EthAddress, InitiatorContract, WETH9Contract, WETH9,
};

#[derive(Clone)]
pub struct SetupEthClient {
	rpc_provider: AlloyProvider,
	rpc_port: u16,
	ws_provider: Option<RootProvider<PubSubFrontend>>,
	config: Config,
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

pub async fn initialize_initiator_contract(
	&self,
	weth: EthAddress,
	owner: EthAddress,
) -> Result<(), anyhow::Error> {
	let call = self.initiator_contract.initialize(weth.0, owner.0);
	send_transaction(call.to_owned(), &send_transaction_rules(), RETRIES, GAS_LIMIT)
		.await
		.expect("Failed to send transaction");
	Ok(())
}
