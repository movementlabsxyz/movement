use alloy_primitives::{address, Address as EthAddress};
use anyhow::anyhow;
use alloy::{
	network::EthereumWallet, node_bindings::Anvil, primitives::U256, providers::ProviderBuilder,
	signers::local::PrivateKeySigner, sol,
};
use alloy_provider
use bridge_shared::bridge_contracts::{BridgeContractInitiator, BridgeContractResult};
use bridge_shared::types::{Amount, BridgeTransferDetails, BridgeTransferId, HashLock, HashLockPreImage, InitiatorAddress, RecipientAddress, TimeLock};

//Codegen the ABI file
sol!("../../../contracts/out/AtomicBridgeInitator.sol/AtomicBridgeInitiator.json",);

pub struct EthHash(); // Alloy type inside

pub struct EthClient {
	provider: Provider,
	anvil: Anvil,
	initiator: AtomicBridgeInitiator,
}

struct AtomicBridgeInitiator(_, ReqwestProvider);

impl EthClient {
	pub fn build(
		url: &str,
		initiator_address: &str,
		counterparty_address: &str,
	) -> Result<Self, anyhow::Error> {
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
