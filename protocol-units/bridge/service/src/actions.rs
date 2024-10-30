use anyhow::anyhow;
use aptos_sdk::rest_client::{Client as RestClient, FaucetClient};
use aptos_sdk::types::account_address::AccountAddress;
use crate::chains::bridge_contracts::BridgeContract;
use crate::chains::bridge_contracts::BridgeContractError;
use crate::types::Amount;
use crate::types::BridgeAddress;
use crate::types::BridgeTransferId;
use crate::types::HashLock;
use crate::types::HashLockPreImage;
use crate::ChainId;
use std::fmt;
use std::future::Future;
use std::pin::Pin;
use thiserror::Error;

use url::Url;

#[derive(Error, Debug, Clone)]
pub struct ActionExecError(TransferAction, BridgeContractError);

impl ActionExecError {
	pub fn inner(self) -> (TransferAction, BridgeContractError) {
		(self.0, self.1)
	}
}

impl fmt::Display for ActionExecError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "Action: {}/ Error: {}", self.0, self.1,)
	}
}

#[derive(Debug, Clone)]
pub struct TransferAction {
	pub chain: ChainId,
	pub transfer_id: BridgeTransferId,
	pub kind: TransferActionType,
}
impl fmt::Display for TransferAction {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "Action: {}/{}/{}", self.chain, self.transfer_id, self.kind)
	}
}

#[derive(Debug, Clone)]
pub enum TransferActionType {
	LockBridgeTransfer {
		bridge_transfer_id: BridgeTransferId,
		hash_lock: HashLock,
		initiator: BridgeAddress<Vec<u8>>,
		recipient: BridgeAddress<Vec<u8>>,
		amount: Amount,
	},
	WaitAndCompleteInitiator(u64, HashLockPreImage),
	RefundInitiator,
	TransferDone,
	NoAction,
}

impl fmt::Display for TransferActionType {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let act = match self {
			TransferActionType::LockBridgeTransfer { .. } => "LockBridgeTransfer",
			TransferActionType::WaitAndCompleteInitiator(..) => "WaitAndCompleteInitiator",
			TransferActionType::RefundInitiator => "RefundInitiator",
			TransferActionType::TransferDone => "TransferDone",
			TransferActionType::NoAction => "NoAction",
		};
		write!(f, "{}", act)
	}
}

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

				// Check whether recipient is a Movement address (32 bytes)
				if recipient.0.len() == 32 {
					tracing::info!("Recipient is a Movement address. Creating faucet client...");
					let faucet_url = Url::parse("https://faucet.testnet.bardock.movementnetwork.xyz")
					.expect("Invalid faucet URL");
					let rest_url = Url::parse("https://testnet.bardock.movementnetwork.xyz")
					.expect("Invalid REST URL");
		
					// Create FaucetClient and RestClient
					let rest_client = RestClient::new(rest_url.clone());
					let faucet_client = FaucetClient::new(faucet_url, rest_url);
		
					let recipient_address: [u8; 32] = recipient.0.clone().try_into()
					.map_err(|_| ActionExecError(action.clone(), BridgeContractError::SerializationError))?;

					let account_address = AccountAddress::new(recipient_address);

					/// Fetch and log balance before funding
					/// This should return an error for a new address because funding creates the account
					match rest_client.get_account_balance(account_address).await {
						Ok(balance) => tracing::info!("Balance before funding: {:?}", balance),
						Err(e) => tracing::error!("Failed to retrieve balance before funding: {:?}", e),
					}
					
					// Execute the funding transaction and capture the result
					match faucet_client
						.fund(account_address, 100_000_000)
						.await
					{
						Ok(tx_result) => {
						tracing::info!("Successfully funded Movement address. Transaction result: {:?}", tx_result);
						}
						Err(e) => {
						tracing::error!("Failed to fund Movement address: {:?}", e);
						return Err(ActionExecError(action.clone(), BridgeContractError::SerializationError));
						}
					};

					// Fetch and log balance before funding
					match rest_client.get_account_balance(account_address).await {
						Ok(balance) => tracing::info!("Balance after funding: {:?}", balance),
						Err(e) => tracing::error!("Failed to retrieve balance before funding: {:?}", e),
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
