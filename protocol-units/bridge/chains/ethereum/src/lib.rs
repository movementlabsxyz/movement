use anyhow::Context;
use std::fmt::Debug;

use alloy::signers::local::PrivateKeySigner;
use alloy_network::EthereumWallet;
use alloy_primitives::private::serde::{Deserialize, Serialize};
use alloy_primitives::{FixedBytes, U256};
use alloy_provider::{Provider, ProviderBuilder};
use alloy_rlp::{Decodable, RlpDecodable, RlpEncodable};
use alloy_sol_types::sol;
use bridge_shared::bridge_contracts::{
	BridgeContractInitiator, BridgeContractInitiatorError, BridgeContractInitiatorResult,
};
use bridge_shared::types::{
	Amount, BridgeTransferDetails, BridgeTransferId, HashLock, HashLockPreImage, InitiatorAddress,
	RecipientAddress, TimeLock,
};

pub mod types;
pub mod utils;

use crate::types::{EthAddress, EthHash};

const INITIATOR_CONTRACT: &str = "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266";
const DEFAULT_GAS_LIMIT: u64 = 10_000_000_000;

///Configuration for the Ethereum Bridge Client
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
	pub rpc_url: Option<String>,
	pub ws_url: Option<String>,
	pub chain_id: String,
	pub signer_private_key: String,
	pub initiator_contract: EthAddress,
	pub gas_limit: u64,
}

impl Default for Config {
	fn default() -> Self {
		Config {
			rpc_url: Some("http://localhost:8545".to_string()),
			ws_url: Some("ws://localhost:8545".to_string()),
			chain_id: "31337".to_string(),
			signer_private_key: Self::default_for_private_key(),
			initiator_contract: EthAddress::from(INITIATOR_CONTRACT.to_string()),
			gas_limit: DEFAULT_GAS_LIMIT,
		}
	}
}

impl Config {
	fn default_for_private_key() -> String {
		let random_wallet = PrivateKeySigner::random();
		random_wallet.to_bytes().to_string()
	}
}

// Codegen from the abi
sol!(
	#[allow(missing_docs)]
	#[sol(rpc)]
	AtomicBridgeInitiator,
	"abis/AtomicBridgeInitiator.json"
);

#[derive(RlpDecodable, RlpEncodable)]
struct EthBridgeTransferDetails {
	pub amount: U256,
	pub originator: EthAddress,
	pub recipient: [u8; 32],
	pub hash_lock: [u8; 32],
	pub time_lock: U256,
	pub state: u8, // Assuming the enum is u8 for now..
}

pub struct EthClient<P> {
	rpc_provider: P,
	initiator_contract: EthAddress,
}

impl EthClient<utils::AlloyProvider> {
	pub async fn build_with_config(config: Config) -> Result<Self, anyhow::Error> {
		let signer = config.signer_private_key.parse::<PrivateKeySigner>()?;
		let rpc_url = config.rpc_url.context("rpc_url not set")?;
		let rpc_provider = ProviderBuilder::new()
			.with_recommended_fillers()
			.wallet(EthereumWallet::from(signer.clone()))
			.on_builtin(&rpc_url)
			.await?;

		Ok(EthClient { rpc_provider, initiator_contract: config.initiator_contract })
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
	type Address = EthAddress;
	type Hash = EthHash;

	async fn initiate_bridge_transfer(
		&mut self,
		// TODO: (richard) we are missing the initiator address here
		_initiator_address: InitiatorAddress<Self::Address>,
		recipient_address: RecipientAddress<Vec<u8>>,
		hash_lock: HashLock<Self::Hash>,
		time_lock: TimeLock,
		amount: Amount,
	) -> BridgeContractInitiatorResult<()> {
		let contract = AtomicBridgeInitiator::new(self.initiator_contract.0, &self.rpc_provider);
		let recipient_bytes: [u8; 32] = recipient_address.0.try_into().unwrap();
		// TODO:(richard) we are missing here the intiator address
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
		// TODO: (richard) the pre-image could be anything really, why the limiting to 32 bytes?
		let generic_error = |desc| BridgeContractInitiatorError::GenericError(String::from(desc));
		let pre_image: [u8; 32] = pre_image
			.0
			.get(0..32)
			.ok_or(generic_error("Could not get required slice from pre-image"))?
			.try_into()
			.map_err(|_| generic_error("Could not convert pre-image to [u8; 32]"))?;

		let contract = AtomicBridgeInitiator::new(self.initiator_contract.0, &self.rpc_provider);
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
		let contract = AtomicBridgeInitiator::new(self.initiator_contract.0, &self.rpc_provider);
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
			.get_storage_at(self.initiator_contract.0, storage_slot)
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
			// TODO: (richard) could these wrappings have some side effects?
			time_lock: TimeLock(eth_details.time_lock.wrapping_to::<u64>()),
			amount: Amount(eth_details.amount.wrapping_to::<u64>()),
		}))
	}
}
