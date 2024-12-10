use super::utils::{self, MovementAddress};
use anyhow::{Context, Result};
use aptos_api_types::{EntryFunctionId, MoveModuleId, ViewRequest};
use aptos_sdk::{
	move_types::identifier::Identifier,
	rest_client::{Client, Response},
	types::LocalAccount,
};
use aptos_types::account_address::AccountAddress;
use bridge_config::common::movement::MovementConfig;
use bridge_util::chains::bridge_contracts::BridgeTransferInitiatedDetails;
use bridge_util::types::Nonce;
use bridge_util::{
	chains::bridge_contracts::{
		BridgeClientContract, BridgeContractError, BridgeContractResult, BridgeRelayerContract,
	},
	types::{Amount, BridgeAddress, BridgeTransferId},
};
use hex;
use std::{str::FromStr, sync::Arc};
use tracing::{debug, info};
use url::Url;

pub const FRAMEWORK_ADDRESS: AccountAddress = AccountAddress::new([
	0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1,
]);

pub const NATIVE_BRIDGE_MODULE_NAME: &str = "native_bridge";

#[allow(dead_code)]
enum Call {
	Lock,
	Complete,
	Abort,
	GetDetails,
}

/// The Client for making calls to the atomic bridge framework modules
#[derive(Clone)]
pub struct MovementClientFramework {
	///Native Address of the
	pub native_address: AccountAddress,
	///The Apotos Rest Client
	pub rest_client: Client,
	///The signer account
	signer: Arc<LocalAccount>,
}

impl MovementClientFramework {
	pub async fn build_with_config(config: &MovementConfig) -> Result<Self, anyhow::Error> {
		let node_connection_url = Url::from_str(config.mvt_rpc_connection_url().as_str())
			.map_err(|_| BridgeContractError::SerializationError)?;

		let rest_client = Client::new(node_connection_url.clone());

		let signer =
			utils::create_local_account(config.movement_signer_key.clone(), &rest_client).await?;
		let native_address = AccountAddress::from_hex_literal(&config.movement_native_address)?;
		Ok(MovementClientFramework { native_address, rest_client, signer: Arc::new(signer) })
	}

	pub async fn build_with_signer(
		signer: LocalAccount,
		config: &MovementConfig,
	) -> Result<Self, anyhow::Error> {
		let node_connection_url = Url::from_str(config.mvt_rpc_connection_url().as_str())
			.map_err(|_| BridgeContractError::SerializationError)?;

		let rest_client = Client::new(node_connection_url.clone());
		let native_address = AccountAddress::from_hex_literal(&config.movement_native_address)?;
		Ok(MovementClientFramework { native_address, rest_client, signer: Arc::new(signer) })
	}

	pub fn rest_client(&self) -> &Client {
		&self.rest_client
	}

	pub fn signer(&self) -> &LocalAccount {
		&self.signer
	}
}

#[async_trait::async_trait]
impl BridgeClientContract<MovementAddress> for MovementClientFramework {
	async fn initiate_bridge_transfer(
		&mut self,
		recipient: BridgeAddress<Vec<u8>>,
		amount: Amount,
	) -> BridgeContractResult<()> {
		tracing::info!("Amount value: {:?}", amount);

		let args = vec![
			utils::serialize_vec_initiator(&recipient.0)?,
			utils::serialize_u64_initiator(*amount)?,
		];

		let payload = utils::make_aptos_payload(
			FRAMEWORK_ADDRESS,
			NATIVE_BRIDGE_MODULE_NAME,
			"initiate_bridge_transfer",
			Vec::new(),
			args,
		);

		let _ = utils::send_and_confirm_aptos_transaction(
			&self.rest_client,
			self.signer.as_ref(),
			payload,
		)
		.await
		.map_err(|_| BridgeContractError::InitiateTransferError)?;

		Ok(())
	}
	
	async fn get_bridge_transfer_details(
		&mut self,
		bridge_transfer_id: BridgeTransferId,
	) -> BridgeContractResult<Option<BridgeTransferInitiatedDetails<MovementAddress>>> {
		let bridge_transfer_id_hex = format!("0x{}", hex::encode(bridge_transfer_id.0));

		let view_request = ViewRequest {
			function: EntryFunctionId {
				module: MoveModuleId {
					address: FRAMEWORK_ADDRESS.clone().into(),
					name: aptos_api_types::IdentifierWrapper(
						Identifier::new("atomic_bridge_store")
							.map_err(|_| BridgeContractError::FunctionViewError)?,
					),
				},
				name: aptos_api_types::IdentifierWrapper(
					Identifier::new("get_bridge_transfer_details_initiator")
						.map_err(|_| BridgeContractError::FunctionViewError)?,
				),
			},
			type_arguments: vec![],
			arguments: vec![serde_json::json!(bridge_transfer_id_hex)],
		};

		let response: Response<Vec<serde_json::Value>> = self
			.rest_client
			.view(&view_request, None)
			.await
			.map_err(|_| BridgeContractError::CallError)?;

		let values = response.inner();

		if values.len() != 1 {
			return Err(BridgeContractError::InvalidResponseLength);
		}

		let value = &values[0];

		let originator_address = AccountAddress::from_hex_literal(
			value["addresses"]["initiator"]
				.as_str()
				.ok_or(BridgeContractError::SerializationError)?,
		)
		.map_err(|_| BridgeContractError::SerializationError)?;

		let recipient_bytes = hex::decode(
			&value["addresses"]["recipient"]["inner"]
				.as_str()
				.ok_or(BridgeContractError::SerializationError)?[2..],
		)
		.map_err(|_| BridgeContractError::SerializationError)?;

		let amount = value["amount"]
			.as_str()
			.ok_or(BridgeContractError::SerializationError)?
			.parse::<u64>()
			.map_err(|_| BridgeContractError::SerializationError)?;

		let time_lock = value["nonce"]
			.as_str()
			.ok_or(BridgeContractError::SerializationError)?
			.parse::<u128>()
			.map_err(|_| BridgeContractError::SerializationError)?;

		let state = value["state"].as_u64().ok_or(BridgeContractError::SerializationError)? as u8;

		let details = BridgeTransferInitiatedDetails {
			bridge_transfer_id,
			initiator: BridgeAddress(MovementAddress(originator_address)),
			recipient: BridgeAddress(recipient_bytes),
			amount: Amount(amount),
			nonce: Nonce(time_lock),
		};

		Ok(Some(details))
	}

	// async fn get_bridge_transfer_details_counterparty(
	// 	&mut self,
	// 	bridge_transfer_id: BridgeTransferId,
	// ) -> BridgeContractResult<Option<BridgeTransferDetailsCounterparty<MovementAddress>>> {
	// 	let bridge_transfer_id_hex = format!("0x{}", hex::encode(bridge_transfer_id.0));

	// 	let view_request = ViewRequest {
	// 		function: EntryFunctionId {
	// 			module: MoveModuleId {
	// 				address: FRAMEWORK_ADDRESS.clone().into(),
	// 				name: aptos_api_types::IdentifierWrapper(
	// 					Identifier::new("atomic_bridge_store")
	// 						.map_err(|_| BridgeContractError::FunctionViewError)?,
	// 				),
	// 			},
	// 			name: aptos_api_types::IdentifierWrapper(
	// 				Identifier::new("get_bridge_transfer_details_counterparty")
	// 					.map_err(|_| BridgeContractError::FunctionViewError)?,
	// 			),
	// 		},
	// 		type_arguments: vec![],
	// 		arguments: vec![serde_json::json!(bridge_transfer_id_hex)],
	// 	};

	// 	let response: Response<Vec<serde_json::Value>> = self
	// 		.rest_client
	// 		.view(&view_request, None)
	// 		.await
	// 		.map_err(|_| BridgeContractError::CallError)?;

	// 	let values = response.inner();

	// 	if values.len() != 1 {
	// 		return Err(BridgeContractError::InvalidResponseLength);
	// 	}

	// 	let value = &values[0];

	// 	let originator_address_bytes = hex::decode(
	// 		&value["addresses"]["initiator"]["inner"]
	// 			.as_str()
	// 			.ok_or(BridgeContractError::SerializationError)?[2..],
	// 	)
	// 	.map_err(|_| BridgeContractError::SerializationError)?;

	// 	let recipient = AccountAddress::from_hex_literal(
	// 		value["addresses"]["recipient"]
	// 			.as_str()
	// 			.ok_or(BridgeContractError::SerializationError)?,
	// 	)
	// 	.map_err(|_| BridgeContractError::SerializationError)?;

	// 	let amount = value["amount"]
	// 		.as_str()
	// 		.ok_or(BridgeContractError::SerializationError)?
	// 		.parse::<u64>()
	// 		.map_err(|_| BridgeContractError::SerializationError)?;

	// 	let hash_lock_array: [u8; 32] = hex::decode(
	// 		&value["hash_lock"].as_str().ok_or(BridgeContractError::SerializationError)?[2..],
	// 	)
	// 	.map_err(|_| BridgeContractError::SerializationError)?
	// 	.try_into()
	// 	.map_err(|_| BridgeContractError::SerializationError)?;

	// 	let time_lock = value["time_lock"]
	// 		.as_str()
	// 		.ok_or(BridgeContractError::SerializationError)?
	// 		.parse::<u64>()
	// 		.map_err(|_| BridgeContractError::SerializationError)?;

	// 	let state = value["state"].as_u64().ok_or(BridgeContractError::SerializationError)? as u8;

	// 	let details = BridgeTransferDetailsCounterparty {
	// 		bridge_transfer_id,
	// 		initiator: BridgeAddress(originator_address_bytes),
	// 		recipient: BridgeAddress(MovementAddress(recipient)),
	// 		amount: Amount(amount),
	// 		hash_lock: HashLock(hash_lock_array),
	// 		time_lock: TimeLock(time_lock),
	// 		state,
	// 	};

	// 	Ok(Some(details))
	// }
}

#[async_trait::async_trait]
impl BridgeRelayerContract<MovementAddress> for MovementClientFramework {
	async fn complete_bridge_transfer(
		&mut self,
		bridge_transfer_id: BridgeTransferId,
		initiator: BridgeAddress<Vec<u8>>,
		recipient: BridgeAddress<MovementAddress>,
		amount: Amount,
		nonce: Nonce,
	) -> BridgeContractResult<()> {
		let args = vec![
			utils::serialize_vec(&bridge_transfer_id.0[..])?,
			utils::serialize_vec_initiator(&initiator.0)?,
			utils::serialize_vec_initiator(&recipient.0)?,
			utils::serialize_u64_initiator(*amount)?,
			utils::serialize_u64_initiator(nonce.0.try_into().unwrap())?,
		];

		info!("The complete_bridge_transfer args are: {:?}", args);

		let payload = utils::make_aptos_payload(
			FRAMEWORK_ADDRESS,
			NATIVE_BRIDGE_MODULE_NAME,
			"complete_bridge_transfer",
			Vec::new(),
			args,
		);

		let result = utils::send_and_confirm_aptos_transaction(
			&self.rest_client,
			self.signer.as_ref(),
			payload,
		)
		.await
		.map_err(|_| BridgeContractError::CompleteTransferError);

		match &result {
			Ok(tx_result) => {
				debug!("Transaction succeeded: {:?}", tx_result);
			}
			Err(err) => {
				debug!("Transaction failed: {:?}", err);
			}
		}

		Ok(())
	}

	async fn get_bridge_transfer_details_with_nonce(
		&mut self,
		nonce: Nonce,
	) -> BridgeContractResult<Option<BridgeTransferInitiatedDetails<MovementAddress>>> {
		todo!()
	}

	async fn is_bridge_transfer_completed(
		&mut self,
		bridge_transfer_id: BridgeTransferId,
	) -> BridgeContractResult<bool> {
		todo!()
	}
}
