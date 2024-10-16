use super::utils::{self, MovementAddress};
use crate::chains::bridge_contracts::BridgeContract;
use crate::chains::bridge_contracts::BridgeContractError;
use crate::chains::bridge_contracts::BridgeContractResult;
use crate::types::BridgeTransferDetailsCounterparty;
use crate::types::{
	Amount, AssetType, BridgeAddress, BridgeTransferDetails, BridgeTransferId, HashLock,
	HashLockPreImage, TimeLock,
};
use anyhow::Result;
use aptos_api_types::{EntryFunctionId, MoveModuleId, ViewRequest};
use aptos_sdk::{
	move_types::identifier::Identifier,
	rest_client::{Client, Response},
	types::LocalAccount,
};
use aptos_types::account_address::AccountAddress;
use bridge_config::common::movement::MovementConfig;
use rand::prelude::*;
use std::str::FromStr;
use std::sync::Arc;
use tracing::{debug, info};
use url::Url;

pub const INITIATOR_MODULE_NAME: &str = "atomic_bridge_initiator";
pub const COUNTERPARTY_MODULE_NAME: &str = "atomic_bridge_counterparty";
const DUMMY_ADDRESS: AccountAddress = AccountAddress::new([0; 32]);

#[allow(dead_code)]
enum Call {
	Lock,
	Complete,
	Abort,
	GetDetails,
}

#[allow(dead_code)]
#[derive(Clone)]
pub struct MovementClient {
	///Native Address of the
	pub native_address: AccountAddress,
	/// Bytes of the non-native (external) chain.
	pub non_native_address: Vec<u8>,
	///The Apotos Rest Client
	pub rest_client: Client,
	///The signer account
	signer: Arc<LocalAccount>,
}

impl MovementClient {
	pub async fn new(config: &MovementConfig) -> Result<Self, anyhow::Error> {
		let node_connection_url = Url::from_str(config.mvt_rpc_connection_url().as_str())
			.map_err(|_| BridgeContractError::SerializationError)?;

		let rest_client = Client::new(node_connection_url.clone());

		let signer =
			utils::create_local_account(config.movement_signer_key.clone(), &rest_client)
				.await?;
		let native_address = AccountAddress::from_hex_literal(&config.movement_native_address)?;
		Ok(MovementClient {
			native_address,
			non_native_address: Vec::new(), //dummy for now
			rest_client,
			signer: Arc::new(signer),
		})
	}

	pub fn rest_client(&self) -> &Client {
		&self.rest_client
	}

	pub fn signer(&self) -> &LocalAccount {
		&self.signer
	}

	pub async fn initiator_set_timelock(
		&mut self,
		time_lock: u64,
	) -> Result<(), BridgeContractError> {
		let args = vec![utils::serialize_u64(&time_lock)?];

		let payload = utils::make_aptos_payload(
			self.native_address,
			"atomic_bridge_initiator",
			"set_time_lock_duration",
			Vec::new(),
			args,
		);

		utils::send_and_confirm_aptos_transaction(&self.rest_client, self.signer.as_ref(), payload)
			.await
			.map_err(|_| BridgeContractError::CallError)?;

		Ok(())
	}

	pub async fn counterparty_set_timelock(
		&mut self,
		time_lock: u64,
	) -> Result<(), BridgeContractError> {
		let args = vec![utils::serialize_u64(&time_lock)?];

		let payload = utils::make_aptos_payload(
			self.native_address,
			"atomic_bridge_counterparty",
			"set_time_lock_duration",
			Vec::new(),
			args,
		);

		utils::send_and_confirm_aptos_transaction(&self.rest_client, self.signer.as_ref(), payload)
			.await
			.map_err(|_| BridgeContractError::CallError)?;

		Ok(())
	}
}

#[async_trait::async_trait]
impl BridgeContract<MovementAddress> for MovementClient {
	async fn initiate_bridge_transfer(
		&mut self,
		_initiator: BridgeAddress<MovementAddress>,
		recipient: BridgeAddress<Vec<u8>>,
		hash_lock: HashLock,
		amount: Amount,
	) -> BridgeContractResult<()> {
		let amount_value = match amount.0 {
			AssetType::Moveth(value) => value,
			_ => return Err(BridgeContractError::ConversionFailed("Amount".to_string())),
		};
		debug!("Amount value: {:?}", amount_value);

		let args = vec![
			utils::serialize_vec_initiator(&recipient.0)?,
			utils::serialize_vec_initiator(&hash_lock.0[..])?,
			utils::serialize_u64_initiator(&amount_value)?,
		];

		let payload = utils::make_aptos_payload(
			self.native_address,
			"atomic_bridge_initiator",
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

	async fn initiator_complete_bridge_transfer(
		&mut self,
		bridge_transfer_id: BridgeTransferId,
		preimage: HashLockPreImage,
	) -> BridgeContractResult<()> {
		let unpadded_preimage = {
			let mut end = preimage.0.len();
			while end > 0 && preimage.0[end - 1] == 0 {
				end -= 1;
			}
			&preimage.0[..end]
		};
		let args2 = vec![
			utils::serialize_vec_initiator(&bridge_transfer_id.0[..])?,
			utils::serialize_vec_initiator(unpadded_preimage)?,
		];

		let payload = utils::make_aptos_payload(
			self.native_address,
			INITIATOR_MODULE_NAME,
			"complete_bridge_transfer",
			Vec::new(),
			args2,
		);

		let _ = utils::send_and_confirm_aptos_transaction(
			&self.rest_client,
			self.signer.as_ref(),
			payload,
		)
		.await
		.map_err(|_| BridgeContractError::CompleteTransferError);

		Ok(())
	}

	async fn counterparty_complete_bridge_transfer(
		&mut self,
		bridge_transfer_id: BridgeTransferId,
		preimage: HashLockPreImage,
	) -> BridgeContractResult<()> {
		let unpadded_preimage = {
			let mut end = preimage.0.len();
			while end > 0 && preimage.0[end - 1] == 0 {
				end -= 1;
			}
			&preimage.0[..end]
		};
		let args2 = vec![
			utils::serialize_vec(&bridge_transfer_id.0[..])?,
			utils::serialize_vec(&unpadded_preimage)?,
		];

		let payload = utils::make_aptos_payload(
			self.native_address,
			COUNTERPARTY_MODULE_NAME,
			"complete_bridge_transfer",
			Vec::new(),
			args2,
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

	async fn lock_bridge_transfer(
		&mut self,
		bridge_transfer_id: BridgeTransferId,
		hash_lock: HashLock,
		initiator: BridgeAddress<Vec<u8>>,
		recipient: BridgeAddress<MovementAddress>,
		amount: Amount,
	) -> BridgeContractResult<()> {
		let amount_value = match amount.0 {
			AssetType::Moveth(value) => value,
			_ => return Err(BridgeContractError::SerializationError),
		};

		let args = vec![
			utils::serialize_vec(&initiator.0)?,
			utils::serialize_vec(&bridge_transfer_id.0[..])?,
			utils::serialize_vec(&hash_lock.0[..])?,
			utils::serialize_vec(&recipient.0)?,
			utils::serialize_u64(&amount_value)?,
		];

		let payload = utils::make_aptos_payload(
			self.native_address,
			COUNTERPARTY_MODULE_NAME,
			"lock_bridge_transfer",
			Vec::new(),
			args,
		);

		let _ = utils::send_and_confirm_aptos_transaction(
			&self.rest_client,
			self.signer.as_ref(),
			payload,
		)
		.await
		.map_err(|_| BridgeContractError::LockTransferError);

		Ok(())
	}

	async fn refund_bridge_transfer(
		&mut self,
		bridge_transfer_id: BridgeTransferId,
	) -> BridgeContractResult<()> {
		let args = vec![utils::serialize_vec_initiator(&bridge_transfer_id.0[..])?];

		let payload = utils::make_aptos_payload(
			self.native_address,
			"atomic_bridge_initiator",
			"refund_bridge_transfer",
			Vec::new(),
			args,
		);

		utils::send_and_confirm_aptos_transaction(&self.rest_client, self.signer.as_ref(), payload)
			.await
			.map_err(|err| BridgeContractError::OnChainError(err.to_string()))?;

		Ok(())
	}

	async fn abort_bridge_transfer(
		&mut self,
		bridge_transfer_id: BridgeTransferId,
	) -> BridgeContractResult<()> {
		let args3 = vec![utils::serialize_vec(&bridge_transfer_id.0[..])?];
		let payload = utils::make_aptos_payload(
			self.native_address,
			COUNTERPARTY_MODULE_NAME,
			"abort_bridge_transfer",
			Vec::new(),
			args3,
		);
		let result = utils::send_and_confirm_aptos_transaction(
			&self.rest_client,
			self.signer.as_ref(),
			payload,
		)
		.await
		.map_err(|_| BridgeContractError::AbortTransferError);

		info!("Abort bridge transfer result: {:?}", &result);

		Ok(())
	}

	async fn get_bridge_transfer_details_initiator(
		&mut self,
		bridge_transfer_id: BridgeTransferId,
	) -> BridgeContractResult<Option<BridgeTransferDetails<MovementAddress>>> {
		let bridge_transfer_id_hex = format!("0x{}", hex::encode(bridge_transfer_id.0));

		let view_request = ViewRequest {
			function: EntryFunctionId {
				module: MoveModuleId {
					address: self.native_address.clone().into(),
					name: aptos_api_types::IdentifierWrapper(
						Identifier::new("atomic_bridge_initiator")
							.map_err(|_| BridgeContractError::FunctionViewError)?,
					),
				},
				name: aptos_api_types::IdentifierWrapper(
					Identifier::new("bridge_transfers")
						.map_err(|_| BridgeContractError::FunctionViewError)?,
				),
			},
			type_arguments: vec![],
			arguments: vec![serde_json::json!(bridge_transfer_id_hex)],
		};

		debug!("View request: {:?}", view_request);

		let response: Response<Vec<serde_json::Value>> = self
			.rest_client
			.view(&view_request, None)
			.await
			.map_err(|_| BridgeContractError::CallError)?;

		let values = response.inner();

		if values.len() != 6 {
			return Err(BridgeContractError::InvalidResponseLength);
		}

		let originator = utils::val_as_str_initiator(values.first())?;
		let recipient = utils::val_as_str_initiator(values.get(1))?;
		let amount = utils::val_as_str_initiator(values.get(2))?
			.parse::<u64>()
			.map_err(|_| BridgeContractError::SerializationError)?;
		let hash_lock = utils::val_as_str_initiator(values.get(3))?;
		let time_lock = utils::val_as_str_initiator(values.get(4))?
			.parse::<u64>()
			.map_err(|_| BridgeContractError::SerializationError)?;
		let state = utils::val_as_u64_initiator(values.get(5))? as u8;

		let originator_address = AccountAddress::from_hex_literal(originator)
			.map_err(|_| BridgeContractError::SerializationError)?;
		let recipient_address_bytes =
			hex::decode(&recipient[2..]).map_err(|_| BridgeContractError::SerializationError)?;
		let hash_lock_array: [u8; 32] = hex::decode(&hash_lock[2..])
			.map_err(|_| BridgeContractError::SerializationError)?
			.try_into()
			.map_err(|_| BridgeContractError::SerializationError)?;

		let details = BridgeTransferDetails {
			bridge_transfer_id,
			initiator_address: BridgeAddress(MovementAddress(originator_address)),
			recipient_address: BridgeAddress(recipient_address_bytes),
			amount: Amount(AssetType::Moveth(amount)),
			hash_lock: HashLock(hash_lock_array),
			time_lock: TimeLock(time_lock),
			state,
		};

		Ok(Some(details))
	}

	async fn get_bridge_transfer_details_counterparty(
		&mut self,
		bridge_transfer_id: BridgeTransferId,
	) -> BridgeContractResult<Option<BridgeTransferDetailsCounterparty<MovementAddress>>> {
		let bridge_transfer_id_hex = format!("0x{}", hex::encode(bridge_transfer_id.0));

		let view_request = ViewRequest {
			function: EntryFunctionId {
				module: MoveModuleId {
					address: self.native_address.clone().into(),
					name: aptos_api_types::IdentifierWrapper(
						Identifier::new("atomic_bridge_counterparty")
							.map_err(|_| BridgeContractError::FunctionViewError)?,
					),
				},
				name: aptos_api_types::IdentifierWrapper(
					Identifier::new("bridge_transfers")
						.map_err(|_| BridgeContractError::FunctionViewError)?,
				),
			},
			type_arguments: vec![],
			arguments: vec![serde_json::json!(bridge_transfer_id_hex)],
		};

		debug!("View request: {:?}", view_request);

		let response: Response<Vec<serde_json::Value>> = self
			.rest_client
			.view(&view_request, None)
			.await
			.map_err(|_| BridgeContractError::CallError)?;

		let values = response.inner();

		if values.len() != 6 {
			return Err(BridgeContractError::InvalidResponseLength);
		}

		let originator = utils::val_as_str_initiator(values.first())?;
		let recipient = utils::val_as_str_initiator(values.get(1))?;
		let amount = utils::val_as_str_initiator(values.get(2))?
			.parse::<u64>()
			.map_err(|_| BridgeContractError::SerializationError)?;
		let hash_lock = utils::val_as_str_initiator(values.get(3))?;
		let time_lock = utils::val_as_str_initiator(values.get(4))?
			.parse::<u64>()
			.map_err(|_| BridgeContractError::SerializationError)?;
		let state = utils::val_as_u64_initiator(values.get(5))? as u8;
		let originator_address_bytes =
			hex::decode(&originator[2..]).map_err(|_| BridgeContractError::SerializationError)?;
		let recipient_address = AccountAddress::from_hex_literal(recipient)
			.map_err(|_| BridgeContractError::SerializationError)?;
			let hash_lock_array: [u8; 32] = hex::decode(&hash_lock[2..])
			.map_err(|_| BridgeContractError::SerializationError)?
			.try_into()
			.map_err(|_| BridgeContractError::SerializationError)?;

		let details = BridgeTransferDetailsCounterparty {
			bridge_transfer_id,
			initiator_address: BridgeAddress(originator_address_bytes),
			recipient_address: BridgeAddress(MovementAddress(recipient_address)),
			amount: Amount(AssetType::Moveth(amount)),
			hash_lock: HashLock(hash_lock_array),
			time_lock: TimeLock(time_lock),
			state,
		};

		Ok(Some(details))
	}
}

use std::process::Stdio;

use tokio::{
	io::{AsyncBufReadExt, BufReader},
	process::Command as TokioCommand,
	sync::oneshot,
	task,
};

impl MovementClient {

	pub async fn new_for_test() -> Result<(Self, tokio::process::Child), anyhow::Error> {
		let kill_cmd = TokioCommand::new("sh")
			.arg("-c")
			.arg("PID=$(ps aux | grep 'movement node run-local-testnet' | grep -v grep | awk '{print $2}' | head -n 1); if [ -n \"$PID\" ]; then kill -9 $PID; fi")
			.output()
			.await?;

		if !kill_cmd.status.success() {
			println!("Failed to kill running movement process: {:?}", kill_cmd.stderr);
		} else {
			println!("Movement process killed if it was running.");
		}

		let delete_dir_cmd = TokioCommand::new("sh")
			.arg("-c")
			.arg("if [ -d '.movement' ]; then rm -rf .movement; fi")
			.output()
			.await?;

		if !delete_dir_cmd.status.success() {
			println!("Failed to delete .movement directory: {:?}", delete_dir_cmd.stderr);
		} else {
			println!(".movement directory deleted if it was present.");
		}

		let (setup_complete_tx, setup_complete_rx) = oneshot::channel();
		let mut child = TokioCommand::new("movement")
			.args(&["node", "run-local-testnet", "--force-restart", "--assume-yes"])
			.stdout(Stdio::piped())
			.stderr(Stdio::piped())
			.spawn()?;

		let stdout = child.stdout.take().expect("Failed to capture stdout");
		let stderr = child.stderr.take().expect("Failed to capture stderr");

		task::spawn(async move {
			let mut stdout_reader = BufReader::new(stdout).lines();
			let mut stderr_reader = BufReader::new(stderr).lines();

			loop {
				tokio::select! {
					line = stdout_reader.next_line() => {
						match line {
							Ok(Some(line)) => {
								println!("STDOUT: {}", line);
								if line.contains("Setup is complete") {
									println!("Testnet is up and running!");
									let _ = setup_complete_tx.send(());
																	return Ok(());
								}
							},
							Ok(_) => {
								return Err(anyhow::anyhow!("Unexpected end of stdout stream"));
							},
							Err(e) => {
								return Err(anyhow::anyhow!("Error reading stdout: {}", e));
							}
						}
					},
					line = stderr_reader.next_line() => {
						match line {
							Ok(Some(line)) => {
								println!("STDERR: {}", line);
								if line.contains("Setup is complete") {
									println!("Testnet is up and running!");
									let _ = setup_complete_tx.send(());
																	return Ok(());
								}
							},
							Ok(_) => {
								return Err(anyhow::anyhow!("Unexpected end of stderr stream"));
							}
							Err(e) => {
								return Err(anyhow::anyhow!("Error reading stderr: {}", e));
							}
						}
					}
				}
			}
		});

		setup_complete_rx.await.expect("Failed to receive setup completion signal");
		println!("Setup complete message received.");

		let node_connection_url = "http://127.0.0.1:8080".to_string();
		let node_connection_url = Url::from_str(node_connection_url.as_str())
			.map_err(|_| BridgeContractError::SerializationError)?;
		let rest_client = Client::new(node_connection_url.clone());

		let mut rng = ::rand::rngs::StdRng::from_seed([3u8; 32]);
		Ok((
			MovementClient {
				native_address: DUMMY_ADDRESS,
				non_native_address: Vec::new(),
				rest_client,
				signer: Arc::new(LocalAccount::generate(&mut rng)),
			},
			child,
		))
	}
}
