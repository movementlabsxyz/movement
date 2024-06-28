use alloy::pubsub::PubSubFrontend;
use alloy_network::{Ethereum, EthereumSigner};
use alloy_primitives::private::serde::{Deserialize, Serialize};
use alloy_primitives::{Address as EthAddress, FixedBytes, U256};
use alloy_provider::{
	fillers::{ChainIdFiller, FillProvider, GasFiller, JoinFill, NonceFiller, SignerFiller},
	Provider, ProviderBuilder, RootProvider,
};
use alloy_signer_wallet::LocalWallet;
use alloy_sol_types::sol;
use alloy_transport::BoxTransport;
use alloy_transport_ws::WsConnect;
use bridge_shared::bridge_contracts::{
	BridgeContractError, BridgeContractInitiator, BridgeContractResult,
};
use bridge_shared::types::{
	Amount, BridgeTransferDetails, BridgeTransferId, HashLock, HashLockPreImage, InitiatorAddress,
	RecipientAddress, TimeLock,
};
use mcr_settlement_client::send_eth_tx::{
	send_tx, InsufficentFunds, SendTxErrorRule, UnderPriced, VerifyRule,
};

use anyhow::Context;

const INITIATOR_ADDRESS: &str = "0xinitiator";
const COUNTERPARTY_ADDRESS: &str = "0xcounter";
const DEFAULT_GAS_LIMIT: u64 = 10_000_000_000;
const MAX_RETRIES: u32 = 5;

///Configuration for the Ethereum Bridge Client
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
	pub rpc_url: Option<String>,
	pub ws_url: Option<String>,
	pub signer_private_key: String,
	pub initiator_address: String,
	pub counterparty_address: Option<String>,
	pub gas_limit: u64,
	pub num_tx_send_retries: u32,
}

impl Default for Config {
	fn default() -> Self {
		Config {
			rpc_url: Some("http://localhost:8545".to_string()),
			ws_url: Some("ws://localhost:8546".to_string()),
			signer_private_key: LocalWallet::random().to_bytes().to_string(),
			initiator_address: Some(INITIATOR_ADDRESS.to_string()),
			counterparty_address: Some(COUNTERPARTY_ADDRESS.to_string()),
			gas_limit: DEFAULT_GAS_LIMIT,
			num_tx_send_retries: MAX_RETRIES,
		}
	}
}

// Codegen from the abi
sol!(
	#[allow(missing_docs)]
	#[sol(rpc)]
	AtomicBridgeInitiator,
	"abis/AtomicBridgeInitiator.json"
);

type AlloyProvider = FillProvider<
	JoinFill<
		JoinFill<
			JoinFill<JoinFill<alloy_provider::Identity, GasFiller>, NonceFiller>,
			ChainIdFiller,
		>,
		SignerFiller<EthereumSigner>,
	>,
	RootProvider<BoxTransport>,
	BoxTransport,
	Ethereum,
>;

#[derive(Debug, Hash, Eq, PartialEq, Clone, Serialize, Deserialize)]
pub struct EthHash([u8; 32]);

pub struct EthClient<P> {
	rpc_provider: P,
	ws_provider: RootProvider<PubSubFrontend>,
	initiator_address: EthAddress,
	counterparty_address: EthAddress,
	send_tx_error_rules: Vec<Box<dyn VerifyRule>>,
	gas_limit: u64,
	num_tx_send_retries: u32,
}

impl EthClient<AlloyProvider> {
	pub async fn build_with_config(
		config: Config,
		counterparty_address: &str,
	) -> Result<Self, anyhow::Error> {
		let signer_private_key = config.signer_private_key;
		let signer: LocalWallet = signer_private_key.parse()?;
		let initiator_address = config.initiator_address.parse()?;
		let rpc_url = config.rpc_url.context("rpc_url not set")?;
		let ws_url = config.ws_url.context("ws_url not set")?;
		let rpc_provider = ProviderBuilder::new()
			.with_recommended_fillers()
			.signer(EthereumSigner::from(signer))
			.on_builtin(&rpc_url)
			.await?;

		EthClient::build_with_provider(
			rpc_provider,
			ws_url,
			initiator_address,
			counterparty_address.parse()?,
			counterparty_address.parse()?,
			config.gas_limit,
			config.num_tx_send_retries,
		)
		.await
	}

	async fn build_with_provider<S>(
		rpc_provider: AlloyProvider,
		ws_provider: S,
		signer_address: EthAddress,
		initiator_address: EthAddress,
		counterparty_address: EthAddress,
		gas_limit: u64,
		num_tx_send_retries: u32,
	) -> Result<Self, anyhow::Error>
	where
		S: Into<String>,
	{
		let ws = WsConnect::new(ws_provider);
		let ws_provider = ProviderBuilder::new().on_ws(ws).await?;

		let rule1: Box<dyn VerifyRule> = Box::new(SendTxErrorRule::<UnderPriced>::new());
		let rule2: Box<dyn VerifyRule> = Box::new(SendTxErrorRule::<InsufficentFunds>::new());
		let send_tx_error_rules = vec![rule1, rule2];

		Ok(EthClient {
			rpc_provider,
			ws_provider,
			initiator_address,
			counterparty_address,
			send_tx_error_rules,
			gas_limit,
			num_tx_send_retries,
		})
	}
}

impl<P> Clone for EthClient<P> {
	fn clone(&self) -> Self {
		todo!()
	}
}

#[async_trait::async_trait]
impl<P> BridgeContractInitiator for EthClient<P>
where
	P: Provider + Clone + Send + Sync + Unpin,
{
	type Address = [u8; 32];
	type Hash = [u8; 32];

	async fn initiate_bridge_transfer(
		&mut self,
		_initiator_address: InitiatorAddress<Self::Address>,
		recipient_address: RecipientAddress<Self::Address>,
		hash_lock: HashLock<Self::Hash>,
		time_lock: TimeLock,
		amount: Amount,
	) -> BridgeContractResult<()> {
		let contract = AtomicBridgeInitiator::new(self.initiator_address, &self.rpc_provider);
		let call = contract.initiateBridgeTransfer(
			U256::from(amount.0),
			FixedBytes(recipient_address.0),
			FixedBytes(hash_lock.0),
			U256::from(time_lock.0),
		);
		send_tx(call, &self.send_tx_error_rules, self.num_tx_send_retries, self.gas_limit as u128)
			.await;
		Ok(())
	}

	async fn complete_bridge_transfer(
		&mut self,
		bridge_transfer_id: BridgeTransferId<Self::Hash>,
		pre_image: HashLockPreImage,
	) -> BridgeContractResult<()> {
		let pre_image: [u8; 32] =
			vec_to_array(pre_image.0).unwrap_or_else(|_| panic!("Failed to convert pre_image"));
		let contract =
			AtomicBridgeInitiator::new(self.initiator_address, &self.rpc_provider.clone());
		let call = contract
			.completeBridgeTransfer(FixedBytes(bridge_transfer_id.0), FixedBytes(pre_image));
		send_tx(call, &self.send_tx_error_rules, self.num_tx_send_retries, self.gas_limit as u128)
			.await;
		Ok(())
	}

	async fn refund_bridge_transfer(
		&mut self,
		bridge_transfer_id: BridgeTransferId<Self::Hash>,
	) -> BridgeContractResult<()> {
		let contract =
			AtomicBridgeInitiator::new(self.initiator_address, &self.rpc_provider.clone());
		let call = contract.refundBridgeTransfer(FixedBytes(bridge_transfer_id.0));
		send_tx(call, &self.send_tx_error_rules, self.num_tx_send_retries, self.gas_limit as u128)
			.await;
		Ok(())
	}

	async fn get_bridge_transfer_details(
		&mut self,
		bridge_transfer_id: BridgeTransferId<Self::Hash>,
	) -> BridgeContractResult<Option<BridgeTransferDetails<Self::Hash, Self::Address>>> {
		let contract =
			AtomicBridgeInitiator::new(self.initiator_address, &self.rpc_provider.clone());
		// look up the Details by Id
		Ok(None)
	}
}

fn vec_to_array(vec: Vec<u8>) -> Result<[u8; 32], &'static str> {
	if vec.len() == 32 {
		// Try to convert the Vec<u8> to [u8; 32]
		match vec.try_into() {
			Ok(array) => Ok(array),
			Err(_) => Err("Failed to convert Vec<u8> to [u8; 32]"),
		}
	} else {
		Err("Vec<u8> does not have exactly 32 elements")
	}
}

mod tests {}
