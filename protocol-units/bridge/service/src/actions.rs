//use crate::chains::movement::utils as movement_utils;
use bridge_util::chains::bridge_contracts::BridgeContractError;
use bridge_util::chains::bridge_contracts::BridgeRelayerContract;
use bridge_util::chains::AddressVecCodec;
use bridge_util::types::BridgeAddress;
use bridge_util::ActionExecError;
use bridge_util::TransferAction;
use bridge_util::TransferActionType;
use std::future::Future;
use std::pin::Pin;

pub fn process_action<A>(
	action: TransferAction,
	mut client: impl BridgeRelayerContract<A> + 'static,
) -> Option<Pin<Box<dyn Future<Output = Result<(), ActionExecError>> + Send>>>
where
	A: Clone + Send + AddressVecCodec,
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
				tracing::info!("Before client.complete_bridge_transfer");
				client
					.complete_bridge_transfer(
						bridge_transfer_id,
						initiator,
						BridgeAddress(A::try_decode_recipient(recipient.0).map_err(|err| {
							ActionExecError(
								action.clone(),
								BridgeContractError::BadAddressEncoding(format!("Complete bridge transfer fail to convert recipient address to vec<u8> : {err}")),
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
					let _ = tokio::time::sleep(tokio::time::Duration::from_secs(wait_time_sec));
				}
				client
					.complete_bridge_transfer(
						bridge_transfer_id,
						initiator,
						BridgeAddress(A::try_decode(recipient.0).map_err(|err| {
							ActionExecError(
								action.clone(),
								BridgeContractError::BadAddressEncoding(format!("Complete bridge transfer fail to convert recipient address to vec<u8> : {err}")),
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
		TransferActionType::CompletedRemoveState => None, //TODO
		TransferActionType::NoAction => None,
	}
}
