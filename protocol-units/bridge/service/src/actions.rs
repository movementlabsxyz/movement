use crate::chains::movement::utils as movement_utils;
use bridge_util::chains::bridge_contracts::BridgeContract;
use bridge_util::chains::bridge_contracts::BridgeContractError;
use bridge_util::types::BridgeAddress;
use bridge_util::ActionExecError;
use bridge_util::TransferAction;
use bridge_util::TransferActionType;
use std::future::Future;
use std::pin::Pin;

pub fn process_action<A>(
	action: TransferAction,
	mut client: impl BridgeContract<A> + 'static,
) -> Option<Pin<Box<dyn Future<Output = Result<(), ActionExecError>> + Send>>>
where
	A: Clone + Send + TryFrom<Vec<u8>>,
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
						BridgeAddress(recipient.0.try_into().map_err(|_| {
							ActionExecError(
								action.clone(),
								BridgeContractError::BadAddressEncoding("lock bridge traénsfer fail to convert recipient address to vec<u8>".to_string()),
							)
						})?),
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
