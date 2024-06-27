use alloy_primitives::{address, Address as EthAddress};
use alloy_primitives::private::serde::{Deserialize, Serialize};
use alloy_provider::{ProviderBuilder};
use bridge_shared::bridge_contracts::{BridgeContractInitiator, BridgeContractResult};
use bridge_shared::types::{Amount, BridgeTransferDetails, BridgeTransferId, HashLock, HashLockPreImage, InitiatorAddress, RecipientAddress, TimeLock};
use alloy_signer_wallet::LocalWallet;
use alloy_sol_types::sol;
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
	pub signer_private_key: Option<String>,
	pub initiator_address: Option<String>,
	pub counterparty_address: Option<String>,
	pub gas_limit: u64,
	pub num_tx_send_retries: u32,
}

impl Default for Config {
	fn default() -> Self {
		Config {
			rpc_url: Some("http://localhost:8545".to_string()),
			ws_url: Some("ws://localhost:8546".to_string()),
			signer_private_key: Some(LocalWallet::random().to_bytes().to_string()),
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
	"../../../contracts/out/AtomicBridgeInitator.sol/AtomicBridgeInitiator.json"
);

pub struct EthHash(); // Alloy type inside

pub struct EthClient {
	rpc_provider: Provider,
	initiator: AtomicBridgeInitiator,
}

impl EthClient {
	pub fn build_with_config(
		config: Config
		counterparty_address: &str,
	) -> Result<Self, anyhow::Error> {
		let signer_private_key = config.signer_private_key.context("signer_private_key not set")?;
		let signer: LocalWallet = signer_private_key.parse()?;
		let anvil = Anvil::new().fork(url).try_spawn()?;
		let rpc_url = anvil.rpc_url()?;
		let provider = ProviderBuilder::new().on_http(rpc_url);
		let initiator = AtomicBridgeInitiator::new(initiator_address.parse()?, provider);
		Ok(Self { provider, anvil, initiator })
	}
}

impl Clone for EthClient {
	fn clone(&self) -> Self {
		todo!()
	}
}

impl BridgeContractInitiator for EthClient {
	type Address = EthAddress;
	type Hash = EthHash;

	async fn initiate_bridge_transfer(
		&mut self,
		_initiator_address: InitiatorAddress<Self::Address>,
		recipient_address: RecipientAddress<Self::Address>,
		hash_lock: HashLock<Self::Hash>,
		time_lock: TimeLock,
		amount: Amount,
	) -> BridgeContractResult<()> {
		// do we for sure not want to do anything with the return from the contract?
		let _res = self
			.initiator
			.initiateBridgeTransfer(amount, recipient_address, hash_lock, time_lock)
			.await?;
		Ok(())
	}

	async fn complete_bridge_transfer(
		&mut self,
		bridge_transfer_id: BridgeTransferId<Self::Hash>,
		pre_image: HashLockPreImage,
	) -> BridgeContractResult<()> {
		let _res = self.initiator.completeBridgeTransfer(bridge_transfer_id, pre_image).await?;
		Ok(())
	}

	async fn refund_bridge_transfer(
		&mut self,
		bridge_transfer_id: BridgeTransferId<Self::Hash>,
	) -> BridgeContractResult<()> {
		Ok(())
	}

	async fn get_bridge_transfer_details(
		&mut self,
		bridge_transfer_id: BridgeTransferId<Self::Hash>,
	) -> BridgeContractResult<Option<BridgeTransferDetails<Self::Hash, Self::Address>>> {
		Ok(None)
	}
}

mod tests {}
