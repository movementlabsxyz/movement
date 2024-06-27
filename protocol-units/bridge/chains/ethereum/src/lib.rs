use alloy::providers::{Proverder, ProviderBuilder};
use bridge_shared::bridge_contracts::BridgeContractInitiator;

pub struct EthHash(); // Alloy type inside

pub struct EthClient {
	provider: Provider,
}

impl EthClient {
		pub fn build(rpc_url: &str) -> Self {
				let provider = ProviderBuilder::new().rpc_url(rpc_url).build();
				Self { provider }
		}
}

impl BridgeContractInitiator for EthClient {
		type Address = EthAddress;
		type Hash = EthHash;

		async fn initiate_bridge_transfer(
			&mut self,
			initiator_address: InitiatorAddress<Self::Address>,
			recipient_address: RecipientAddress<Self::Address>,
			hash_lock: HashLock<Self::Hash>,
			time_lock: TimeLock,
			amount: Amount,
		) -> BridgeContractResult<()> {
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
		) -> BridgeContractResult<Option<BridgeTransferDetails<Self::Hash, Self::Address>> {
			Ok(None)
		}
}



