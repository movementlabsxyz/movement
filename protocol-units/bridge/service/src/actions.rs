//use crate::chains::movement::utils as movement_utils;
use crate::runtime::Runtime;
use bridge_util::chains::bridge_contracts::BridgeContractError;
use bridge_util::chains::bridge_contracts::BridgeRelayerContract;
use bridge_util::types::BridgeAddress;
use bridge_util::ActionExecError;
use bridge_util::TransferAction;
use bridge_util::TransferActionType;
use std::future::Future;
use std::pin::Pin;

pub fn process_action<A>(
	action: TransferAction,
	state_runtime: &mut Runtime,
	mut client: impl BridgeRelayerContract<A> + 'static,
) -> Option<Pin<Box<dyn Future<Output = Result<(), ActionExecError>> + Send>>>
where
	A: Clone + Send + TryFrom<Vec<u8>>,
{
	tracing::info!("Action: creating execution for action:{action}");
	match action.kind.clone() {
		TransferActionType::CompleteBridgeTransfer {
			bridge_transfer_id,
			initiator,
			recipient,
			amount,
			nonce,
		} => {
			let future = async move {
				client
					.complete_bridge_transfer(
						bridge_transfer_id,
						initiator,
						BridgeAddress(recipient.0.try_into().map_err(|_| {
							ActionExecError(
								action.clone(),
								BridgeContractError::BadAddressEncoding("Complete bridge transfer fail to convert recipient address to vec<u8>".to_string()),
							)
						})?),
						amount,
						nonce,
					)
					.await
					.map_err(|err| ActionExecError(action, err))
			};
			Some(Box::pin(future))
		}
		TransferActionType::AbortedReplay {
			bridge_transfer_id,
			initiator,
			recipient,
			amount,
			nonce,
			wait_time_sec,
		} => {
			let future = async move {
				if wait_time_sec != 0 {
					let _ =
						tokio::time::sleep(tokio::time::Duration::from_secs(wait_time_sec)).await;
				}
				client
					.complete_bridge_transfer(
						bridge_transfer_id,
						initiator,
						BridgeAddress(recipient.0.try_into().map_err(|_| {
							ActionExecError(
								action.clone(),
								BridgeContractError::BadAddressEncoding("lock bridge transfer fail to convert recipient address to vec<u8>".to_string()),
							)
						})?),
						amount,
						nonce,
					)
					.await
					.map_err(|err| ActionExecError(action, err))
			};
			Some(Box::pin(future))
		}
		TransferActionType::CompletedRemoveState => {
			state_runtime.remove_transfer(action.transfer_id);
			None
		}
		TransferActionType::NoAction => None,
	}
}
