use crate::chains::movement::utils as movement_utils;
use bridge_util::actions::*;
use bridge_util::chains::bridge_contracts::BridgeContract;
use bridge_util::types::BridgeAddress;
use std::future::Future;
use std::pin::Pin;

pub fn process_action<A>(
	action: TransferAction,
	mut client: impl BridgeContract<A> + 'static,
) -> Option<Pin<Box<dyn Future<Output = Result<(), ActionExecError>> + Send>>>
where
	A: Clone + Send + From<Vec<u8>>,
{
	tracing::info!("Action: creating execution for action:{action}");
	match action.kind.clone() {
		TransferActionType::LockBridgeTransfer {
			bridge_transfer_id,
			hash_lock,
			initiator,
			recipient,
			amount,
		} => {
			let future = async move {
				if recipient.0.len() == 32 {
					if let Err(e) = movement_utils::fund_recipient(&recipient).await {
						return Err(ActionExecError(action.clone(), e));
					}
				}

				client
					.lock_bridge_transfer(
						bridge_transfer_id,
						hash_lock,
						initiator,
						BridgeAddress(recipient.0.into()),
						amount,
					)
					.await
					.map_err(|err| ActionExecError(action, err))
			};
			Some(Box::pin(future))
		}
		TransferActionType::WaitAndCompleteInitiator(wait_time_sec, secret) => {
			let future = async move {
				if wait_time_sec != 0 {
					let _ = tokio::time::sleep(tokio::time::Duration::from_secs(wait_time_sec));
				}
				client
					.initiator_complete_bridge_transfer(action.transfer_id, secret)
					.await
					.map_err(|err| ActionExecError(action, err))
			};
			Some(Box::pin(future))
		}
		TransferActionType::RefundInitiator => None,
		TransferActionType::TransferDone => None,
		TransferActionType::NoAction => None,
	}
}
