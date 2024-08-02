use alloy_rlp::Decodable;
use serde_with::serde_as;
use std::fmt::Debug;
use url::Url;

use alloy::primitives::{private::serde::Deserialize, Address, FixedBytes, U256};
use alloy::providers::{Provider, ProviderBuilder, RootProvider, WsConnect};
use alloy::{
	network::EthereumWallet,
	rlp::{RlpDecodable, RlpEncodable},
};
use alloy::{pubsub::PubSubFrontend, signers::local::PrivateKeySigner};
use bridge_shared::bridge_contracts::{
	BridgeContractCounterparty, BridgeContractCounterpartyError, BridgeContractCounterpartyResult,
	BridgeContractInitiator, BridgeContractInitiatorError, BridgeContractInitiatorResult,
};
use bridge_shared::types::{
	Amount, BridgeTransferDetails, BridgeTransferId, HashLock, HashLockPreImage, InitiatorAddress,
	RecipientAddress, TimeLock,
};

pub mod types;
pub mod utils;

use crate::types::{EthAddress, EthHash};

// Codegen from the abis
alloy::sol!(
	#[allow(missing_docs)]
	#[sol(rpc)]
	AtomicBridgeInitiator,
	"abis/AtomicBridgeInitiator.json"
);

alloy::sol!(
	#[allow(missing_docs)]
	#[sol(rpc)]
	AtomicBridgeCounterparty,
	"abis/AtomicBridgeCounterparty.json"
);

///Configuration for the Ethereum Bridge Client
#[serde_as]
#[derive(Clone, Debug, Deserialize)]
pub struct Config {
	pub rpc_url: Url,
	pub ws_url: Url,
	#[serde_as(as = "serde_with::DisplayFromStr")]
	pub signer_private_key: PrivateKeySigner,
	pub initiator_contract: Option<Address>,
	pub counterparty_contract: Option<Address>,
	pub gas_limit: u64,
}

impl Config {
	pub fn build_for_test() -> Self {
		Config {
			rpc_url: "http://localhost:8545".parse().unwrap(),
			ws_url: "ws://localhost:8545".parse().unwrap(),
			signer_private_key: PrivateKeySigner::random(),
			initiator_contract: None,
			counterparty_contract: None,
			gas_limit: 10_000_000_000,
		}
	}
}

#[derive(RlpDecodable, RlpEncodable)]
struct EthBridgeTransferDetails {
	pub amount: U256,
	pub originator: EthAddress,
	pub recipient: [u8; 32],
	pub hash_lock: [u8; 32],
	pub time_lock: U256,
	pub state: u8,
}

// We need to be able to build the client and deploy the contracts
// using that clients, therfore the `initiator_contract` and `counterparty_contract`
// should be optional, as their values will be unknown at the time of building the client.
// This is true for the integration tests.
#[derive(Clone)]
pub struct EthClient {
	rpc_provider: utils::AlloyProvider,
	rpc_port: u16,
	ws_provider: Option<RootProvider<PubSubFrontend>>,
	initiator_contract: Option<Address>,
	counterparty_contract: Option<Address>,
	config: Config,
}

impl EthClient {
	pub async fn new(config: impl Into<Config>) -> Result<Self, anyhow::Error> {
		let config = config.into();
		let rpc_provider = ProviderBuilder::new()
			.with_recommended_fillers()
			.wallet(EthereumWallet::from(config.signer_private_key.clone()))
			.on_builtin(config.rpc_url.as_str())
			.await?;

		// let ws = WsConnect::new(ws_url);
		// println!("ws {:?}", ws);
		// let ws_provider = ProviderBuilder::new().on_ws(ws).await?;
		// println!("ws_provider {:?}", ws_provider);
		Ok(EthClient {
			rpc_provider,
			rpc_port: 8545,
			ws_provider: None,
			initiator_contract: config.initiator_contract,
			counterparty_contract: config.counterparty_contract,
			config,
		})
	}

	pub async fn get_block_number(&self) -> Result<u64, anyhow::Error> {
		self.rpc_provider
			.get_block_number()
			.await
			.map_err(|e| anyhow::anyhow!("Failed to get block number: {}", e))
	}

	pub fn get_signer_address(&self) -> Address {
		self.config.signer_private_key.address()
	}

	pub fn rpc_provider(&self) -> utils::AlloyProvider {
		self.rpc_provider.clone()
	}

	pub fn rpc_port(&self) -> u16 {
		self.rpc_port
	}

	pub fn initiator_contract(&self) -> BridgeContractInitiatorResult<Address> {
		match self.initiator_contract {
			Some(contract) => Ok(contract),
			None => Err(BridgeContractInitiatorError::GenericError(
				"Initiator contract address not set".to_string(),
			)),
		}
	}

	pub fn counterparty_contract(&self) -> BridgeContractCounterpartyResult<Address> {
		match self.counterparty_contract {
			Some(contract) => Ok(contract),
			None => Err(BridgeContractCounterpartyError::GenericError(
				"Counterparty contract address not set".to_string(),
			)),
		}
	}
}

#[async_trait::async_trait]
impl BridgeContractInitiator for EthClient {
	type Address = EthAddress;
	type Hash = EthHash;

	// `_initiator_address`, or in the contract, `originator` is set
	// via the `msg.sender`, which is stored in the `rpc_provider`.
	// So `initiator_address` arg is not used here.
	async fn initiate_bridge_transfer(
		&mut self,
		_initiator_address: InitiatorAddress<Self::Address>,
		recipient_address: RecipientAddress<Vec<u8>>,
		hash_lock: HashLock<Self::Hash>,
		time_lock: TimeLock,
		amount: Amount,
	) -> BridgeContractInitiatorResult<()> {
		let contract = AtomicBridgeInitiator::new(self.initiator_contract()?, &self.rpc_provider);
		let recipient_bytes: [u8; 32] =
			recipient_address.0.try_into().expect("Recipient address must be 32 bytes");
		let call = contract.initiateBridgeTransfer(
			U256::from(amount.0),
			FixedBytes(recipient_bytes),
			FixedBytes(hash_lock.0),
			U256::from(time_lock.0),
		);

		utils::send_transaction(call)
			.await
			.map_err(BridgeContractInitiatorError::generic)
			.map(|_| ())
	}

	async fn complete_bridge_transfer(
		&mut self,
		bridge_transfer_id: BridgeTransferId<Self::Hash>,
		pre_image: HashLockPreImage,
	) -> BridgeContractInitiatorResult<()> {
		// The Alloy generated type for smart contract`pre_image` arg is `FixedBytes<32>`
		// so it must be converted to `[u8; 32]`.
		let generic_error = |desc| BridgeContractInitiatorError::GenericError(String::from(desc));
		let pre_image: [u8; 32] = pre_image
			.0
			.get(0..32)
			.ok_or(generic_error("Could not get required slice from pre-image"))?
			.try_into()
			.map_err(|_| generic_error("Could not convert pre-image to [u8; 32]"))?;

		let contract = AtomicBridgeInitiator::new(self.initiator_contract()?, &self.rpc_provider);
		let call = contract
			.completeBridgeTransfer(FixedBytes(bridge_transfer_id.0), FixedBytes(pre_image));

		utils::send_transaction(call)
			.await
			.map_err(BridgeContractInitiatorError::generic)
			.map(|_| ())
	}

	async fn refund_bridge_transfer(
		&mut self,
		bridge_transfer_id: BridgeTransferId<Self::Hash>,
	) -> BridgeContractInitiatorResult<()> {
		let contract = AtomicBridgeInitiator::new(self.initiator_contract()?, &self.rpc_provider);
		let call = contract.refundBridgeTransfer(FixedBytes(bridge_transfer_id.0));

		utils::send_transaction(call)
			.await
			.map_err(BridgeContractInitiatorError::generic)
			.map(|_| ())
	}

	async fn get_bridge_transfer_details(
		&mut self,
		bridge_transfer_id: BridgeTransferId<Self::Hash>,
	) -> BridgeContractInitiatorResult<Option<BridgeTransferDetails<Self::Address, Self::Hash>>> {
		let generic_error = |desc| BridgeContractInitiatorError::GenericError(String::from(desc));

		let mapping_slot = U256::from(0); // the mapping is the zeroth slot in the contract
		let key = bridge_transfer_id.0;
		let storage_slot = utils::calculate_storage_slot(key, mapping_slot);
		let storage: U256 = self
			.rpc_provider
			.get_storage_at(self.initiator_contract()?, storage_slot)
			.await
			.map_err(|_| generic_error("could not find storage"))?;
		let storage_bytes = storage.to_be_bytes::<32>();
		let mut storage_slice = &storage_bytes[..];
		let eth_details = EthBridgeTransferDetails::decode(&mut storage_slice)
			.map_err(|_| generic_error("could not decode storage"))?;

		Ok(Some(BridgeTransferDetails {
			bridge_transfer_id,
			initiator_address: InitiatorAddress(eth_details.originator),
			recipient_address: RecipientAddress(eth_details.recipient.to_vec()),
			hash_lock: HashLock(eth_details.hash_lock),
			//@TODO unit test these wrapping to check for any nasty side effects.
			time_lock: TimeLock(eth_details.time_lock.wrapping_to::<u64>()),
			amount: Amount(eth_details.amount.wrapping_to::<u64>()),
		}))
	}
}

#[async_trait::async_trait]
impl BridgeContractCounterparty for EthClient {
	type Address = EthAddress;
	type Hash = EthHash;

	async fn lock_bridge_transfer_assets(
		&mut self,
		bridge_transfer_id: BridgeTransferId<Self::Hash>,
		hash_lock: HashLock<Self::Hash>,
		time_lock: TimeLock,
		initiator: InitiatorAddress<Vec<u8>>,
		recipient: RecipientAddress<Self::Address>,
		amount: Amount,
	) -> BridgeContractCounterpartyResult<()> {
		let contract =
			AtomicBridgeCounterparty::new(self.counterparty_contract()?, &self.rpc_provider);
		let initiator: [u8; 32] = initiator.0.try_into().unwrap();
		let call = contract.lockBridgeTransferAssets(
			FixedBytes(initiator),
			FixedBytes(bridge_transfer_id.0),
			FixedBytes(hash_lock.0),
			U256::from(time_lock.0),
			Address::from(recipient.0 .0),
			U256::from(amount.0),
		);
		utils::send_transaction(call)
			.await
			.map_err(BridgeContractCounterpartyError::generic)
			.map(|_| ())
	}

	async fn complete_bridge_transfer(
		&mut self,
		bridge_transfer_id: BridgeTransferId<Self::Hash>,
		secret: HashLockPreImage,
	) -> BridgeContractCounterpartyResult<()> {
		let contract =
			AtomicBridgeCounterparty::new(self.counterparty_contract()?, &self.rpc_provider);
		let secret: [u8; 32] = secret.0.try_into().unwrap();
		let call =
			contract.completeBridgeTransfer(FixedBytes(bridge_transfer_id.0), FixedBytes(secret));
		utils::send_transaction(call)
			.await
			.map_err(BridgeContractCounterpartyError::generic)
			.map(|_| ())
	}

	async fn abort_bridge_transfer(
		&mut self,
		bridge_transfer_id: BridgeTransferId<Self::Hash>,
	) -> BridgeContractCounterpartyResult<()> {
		let contract =
			AtomicBridgeCounterparty::new(self.counterparty_contract()?, &self.rpc_provider);
		let call = contract.abortBridgeTransfer(FixedBytes(bridge_transfer_id.0));
		utils::send_transaction(call)
			.await
			.map_err(BridgeContractCounterpartyError::generic)
			.map(|_| ())
	}

	async fn get_bridge_transfer_details(
		&mut self,
		bridge_transfer_id: BridgeTransferId<Self::Hash>,
	) -> BridgeContractCounterpartyResult<Option<BridgeTransferDetails<Self::Address, Self::Hash>>>
	{
		let generic_error =
			|desc| BridgeContractCounterpartyError::GenericError(String::from(desc));

		let mapping_slot = U256::from(1); // the mapping is the 1st slot in the contract
		let key = bridge_transfer_id.0;
		let storage_slot = utils::calculate_storage_slot(key, mapping_slot);
		let storage: U256 = self
			.rpc_provider
			.get_storage_at(self.counterparty_contract()?, storage_slot)
			.await
			.map_err(|_| generic_error("could not find storage"))?;
		let storage_bytes = storage.to_be_bytes::<32>();
		let mut storage_slice = &storage_bytes[..];
		let eth_details = EthBridgeTransferDetails::decode(&mut storage_slice)
			.map_err(|_| generic_error("could not decode storage"))?;

		Ok(Some(BridgeTransferDetails {
			bridge_transfer_id,
			initiator_address: InitiatorAddress(eth_details.originator),
			recipient_address: RecipientAddress(eth_details.recipient.to_vec()),
			hash_lock: HashLock(eth_details.hash_lock),
			//@TODO unit test these wrapping to check for any nasty side effects.
			time_lock: TimeLock(eth_details.time_lock.wrapping_to::<u64>()),
			amount: Amount(eth_details.amount.wrapping_to::<u64>()),
		}))
	}
}
