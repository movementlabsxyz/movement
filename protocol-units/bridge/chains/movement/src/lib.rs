use aptos_sdk::{move_types::language_storage::TypeTag, rest_client::Client, types::LocalAccount};
use aptos_types::account_address::AccountAddress;
use bridge_shared::{
	bridge_contracts::{
		BridgeContractCounterparty, BridgeContractCounterpartyError,
		BridgeContractCounterpartyResult,
	},
	types::{
		Amount, BridgeTransferDetails, BridgeTransferId, HashLock, HashLockPreImage,
		InitiatorAddress, RecipientAddress, TimeLock,
	},
};
use rand::prelude::*;
use serde::Serialize;
use std::str::FromStr;
use url::Url;

use crate::utils::MovementAddress;

pub mod utils;

const DUMMY_ADDRESS: AccountAddress = AccountAddress::new([0; 32]);
const COUNTERPARTY_MODULE_NAME: &str = "atomic_bridge_counterparty";

enum Call {
	Lock,
	Complete,
	Abort,
	GetDetails,
}

pub struct Config {
	pub rpc_url: Option<String>,
	pub ws_url: Option<String>,
	pub chain_id: String,
	pub signer_private_key: String,
	pub initiator_contract: MovementAddress,
	pub gas_limit: u64,
}

pub struct MovementClient {
	///Address of the counterparty moduke
	counterparty_address: AccountAddress,
	///Address of the initiator module
	initiator_address: Vec<u8>,
	///The Apotos Rest Client
	rest_client: Client,
	///The signer account
	signer: LocalAccount,
}

impl MovementClient {
	pub async fn new(config: Config) -> Result<Self, anyhow::Error> {
		let dot_movement = dot_movement::DotMovement::try_from_env().unwrap();
		let suzuka_config =
			dot_movement.try_get_config_from_json::<suzuka_config::Config>().unwrap();
		let node_connection_address = suzuka_config
			.execution_config
			.maptos_config
			.client
			.maptos_rest_connection_hostname;
		let node_connection_port =
			suzuka_config.execution_config.maptos_config.client.maptos_rest_connection_port;

		let node_connection_url =
			format!("http://{}:{}", node_connection_address, node_connection_port);
		let node_connection_url = Url::from_str(node_connection_url.as_str()).unwrap();

		let rest_client = Client::new(node_connection_url.clone());

		let seed = [3u8; 32];
		let mut rng = rand::rngs::StdRng::from_seed(seed);
		let signer = LocalAccount::generate(&mut rng);

		Ok(MovementClient {
			initiator_address: Vec::new(), //dummy for now
			rest_client,
			counterparty_address: DUMMY_ADDRESS,
			signer,
		})
	}
}

impl Clone for MovementClient {
	fn clone(&self) -> Self {
		todo!()
	}
}

#[async_trait::async_trait]
impl BridgeContractCounterparty for MovementClient {
	type Address = MovementAddress;
	type Hash = [u8; 32];

	async fn lock_bridge_transfer_assets(
		&mut self,
		bridge_transfer_id: BridgeTransferId<Self::Hash>,
		hash_lock: HashLock<Self::Hash>,
		time_lock: TimeLock,
		initiator: InitiatorAddress<Vec<u8>>,
		recipient: RecipientAddress<Self::Address>,
		amount: Amount,
	) -> BridgeContractCounterpartyResult<()> {
		//@TODO properly return an error instead of unwrapping
		let args = vec![
			to_bcs_bytes(&initiator.0).unwrap(),
			to_bcs_bytes(&bridge_transfer_id.0).unwrap(),
			to_bcs_bytes(&hash_lock.0).unwrap(),
			to_bcs_bytes(&time_lock.0).unwrap(),
			to_bcs_bytes(&recipient.0).unwrap(),
			to_bcs_bytes(&amount.0).unwrap(),
		];
		let payload = utils::make_aptos_payload(
			self.counterparty_address,
			COUNTERPARTY_MODULE_NAME,
			"lock_bridge_transfer_assets",
			self.counterparty_type_args(Call::Lock),
			args,
		);
		let _ = utils::send_aptos_transaction(&self.rest_client, &mut self.signer, payload)
			.await
			.map_err(|_| BridgeContractCounterpartyError::LockTransferAssetsError);
		Ok(())
	}

	async fn complete_bridge_transfer(
		&mut self,
		bridge_transfer_id: BridgeTransferId<Self::Hash>,
		preimage: HashLockPreImage,
	) -> BridgeContractCounterpartyResult<()> {
		let args = vec![
			to_bcs_bytes(&self.signer.address()).unwrap(),
			to_bcs_bytes(&bridge_transfer_id.0).unwrap(),
			to_bcs_bytes(&preimage.0).unwrap(),
		];
		let payload = utils::make_aptos_payload(
			self.counterparty_address,
			COUNTERPARTY_MODULE_NAME,
			"complete_bridge_transfer",
			self.counterparty_type_args(Call::Complete),
			args,
		);
		let _ = utils::send_aptos_transaction(&self.rest_client, &mut self.signer, payload)
			.await
			.map_err(|_| BridgeContractCounterpartyError::CompleteTransferError);
		Ok(())
	}

	async fn abort_bridge_transfer(
		&mut self,
		bridge_transfer_id: BridgeTransferId<Self::Hash>,
	) -> BridgeContractCounterpartyResult<()> {
		let args = vec![
			to_bcs_bytes(&self.signer.address()).unwrap(),
			to_bcs_bytes(&bridge_transfer_id.0).unwrap(),
		];
		let payload = utils::make_aptos_payload(
			self.counterparty_address,
			COUNTERPARTY_MODULE_NAME,
			"abort_bridge_transfer",
			self.counterparty_type_args(Call::Abort),
			args,
		);
		let _ = utils::send_aptos_transaction(&self.rest_client, &mut self.signer, payload)
			.await
			.map_err(|_| BridgeContractCounterpartyError::AbortTransferError);
		Ok(())
	}

	async fn get_bridge_transfer_details(
		&mut self,
		_bridge_transfer_id: BridgeTransferId<Self::Hash>,
	) -> BridgeContractCounterpartyResult<Option<BridgeTransferDetails<Self::Address, Self::Hash>>>
	{
		// let _ = utils::send_view_request(
		// 	self.rest_client,
		// 	self.counterparty_address,
		// 	"atomic_bridge_counterparty".to_string(),
		// );
		todo!();
	}
}

impl MovementClient {
	fn counterparty_type_args(&self, call: Call) -> Vec<TypeTag> {
		match call {
			Call::Lock => vec![TypeTag::Address, TypeTag::U64, TypeTag::U64, TypeTag::U8],
			Call::Complete => vec![TypeTag::Address, TypeTag::U64, TypeTag::U8],
			Call::Abort => vec![TypeTag::Address, TypeTag::U64],
			Call::GetDetails => vec![TypeTag::Address, TypeTag::U64],
		}
	}
}

fn to_bcs_bytes<T>(value: &T) -> Result<Vec<u8>, anyhow::Error>
where
	T: Serialize,
{
	Ok(bcs::to_bytes(value)?)
}
