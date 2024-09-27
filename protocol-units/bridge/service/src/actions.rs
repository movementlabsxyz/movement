use crate::chains::bridge_contracts::BridgeContract;
use crate::chains::bridge_contracts::BridgeContractError;
use crate::types::Amount;
use crate::types::BridgeAddress;
use crate::types::BridgeTransferId;
use crate::types::HashLock;
use crate::types::HashLockPreImage;
use crate::types::TimeLock;
use crate::ChainId;
use std::future::Future;
use std::pin::Pin;

pub struct TransferAction<A> {
	pub init_chain: ChainId,
	pub transfer_id: BridgeTransferId,
	pub kind: TransferActionType<A>,
}

pub enum TransferActionType<A> {
	LockBridgeTransfer {
		bridge_transfer_id: BridgeTransferId,
		hash_lock: HashLock,
		initiator: BridgeAddress<Vec<u8>>,
		recipient: BridgeAddress<A>,
		amount: Amount,
	},
	WaitAndCompleteInitiator(u64, HashLockPreImage),
	RefundInitiator,
	TransferDone,
	NoAction,
}

pub fn process_action<A: std::marker::Send + 'static>(
	action: TransferAction<A>,
	mut client: impl BridgeContract<A> + 'static,
) -> Option<Pin<Box<dyn Future<Output = Result<(), BridgeContractError>> + Send>>> {
	match action.kind {
		TransferActionType::LockBridgeTransfer {
			bridge_transfer_id,
			hash_lock,
			initiator,
			recipient,
			amount,
		} => {
			let future = async move {
				client
					.lock_bridge_transfer(
						bridge_transfer_id,
						hash_lock,
						initiator,
						recipient,
						amount,
					)
					.await
			};
			Some(Box::pin(future))
		}
		TransferActionType::WaitAndCompleteInitiator(wait_time_sec, secret) => {
			let future = async move {
				if wait_time_sec != 0 {
					let _ = tokio::time::sleep(tokio::time::Duration::from_secs(wait_time_sec));
				}
				client.initiator_complete_bridge_transfer(action.transfer_id, secret).await
			};
			Some(Box::pin(future))
		}
		TransferActionType::RefundInitiator => None,
		TransferActionType::TransferDone => None,
		TransferActionType::NoAction => None,
	}
}
