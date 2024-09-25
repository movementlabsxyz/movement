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
	init_chain: ChainId,
	transfer_id: BridgeTransferId,
	kind: TransferActionType<A>,
}

pub enum TransferActionType<A> {
	LockBridgeTransfer {
		bridge_transfer_id: BridgeTransferId,
		hash_lock: HashLock,
		time_lock: TimeLock,
		initiator: BridgeAddress<Vec<u8>>,
		recipient: A,
		amount: Amount,
	},
	TransferLocked,
	WaitThenReleaseBurnInitiator(u64, HashLockPreImage),
	RefundInitiator,
	TransferDone,
}

pub fn process_action<A>(
	action: TransferAction<A>,
	mut client: impl BridgeContract<A> + 'static,
) -> Option<
	//    Pin<Box<dyn Future<Output = Result<R, E>> + Send>>
	Pin<Box<dyn Future<Output = Result<(), BridgeContractError>> + Send>>,
> {
	match action.kind {
		TransferActionType::LockBridgeTransfer {
			bridge_transfer_id,
			hash_lock,
			time_lock,
			initiator,
			recipient,
			amount,
		} => {
			// let future = client.lock_bridge_transfer(
			// 	bridge_transfer_id,
			// 	hash_lock,
			// 	time_lock,
			// 	initiator,
			// 	recipient,
			// 	amount,
			// );
			// Some(future)
			None
		}
		TransferActionType::TransferLocked => None,
		TransferActionType::WaitThenReleaseBurnInitiator(wait_time_sec, secret) => {
			let future = async move {
				if wait_time_sec != 0 {
					tokio::time::sleep(tokio::time::Duration::from_secs(wait_time_sec));
				}
				client.complete_bridge_transfer(action.transfer_id, secret).await
			};
			Some(Box::pin(future))
		}
		TransferActionType::RefundInitiator => None,
		TransferActionType::TransferDone => None,
	}
}
