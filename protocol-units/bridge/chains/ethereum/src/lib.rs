use alloy::providers::{Proverder, ProviderBuilder};
use alloy::{node_bindings::Anvil, providers::ProviderBuilder, sol};
use alloy_primitives::{address, Address as EthAddress};
use anyhow::anyhow;
use bridge_shared::bridge_contracts::BridgeContractInitiator;

//Codegen the ABI file
sol!("../../../contracts/out/AtomicBridgeInitator.sol/AtomicBridgeInitiator.json",);

pub struct EthHash(); // Alloy type inside

pub struct EthClient {
	provider: Provider,
	anvil: Anvil,
	initiator: AtomicBridgeInitiator,
}

impl EthClient {
	pub fn build(
		url: &str,
		initiator_address: &str,
		counterparty_address: &str,
	) -> Result<Self, anyhow::Error> {
		let anvil = Anvil::new().fork(rpc_url).try_spawn()?;
		let rpc_url = anvil.rpc_url()?;
		let provider = ProviderBuilder::new().on_http(rpc_url);
		let initiator = AtomicBridgeInitiator::new(initiator_address.parse()?, provider);
		Ok(Self { provider, anvil, initiator })
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
		self.initiator
			.initiateBridgeTransfer(amount, recipient_address, hash_lock, time_lock)
			.await?;
		Ok(())
	}

	async fn complete_bridge_transfer(
		&mut self,
		bridge_transfer_id: BridgeTransferId<Self::Hash>,
		secret: HashLockPreImage,
	) -> BridgeContractResult<()> {
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
