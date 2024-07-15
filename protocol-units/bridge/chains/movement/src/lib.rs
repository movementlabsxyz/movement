use aptos_sdk::{
	rest_client::{
		aptos_api_types::{
			EntryFunctionId, EntryFunctionPayload, IdentifierWrapper, MoveModuleId, MoveType,
		},
		Client, FaucetClient,
	},
	types::{transaction::TransactionPayload, LocalAccount},
};
use aptos_types::{account_address::AccountAddress, transaction::EntryFunction};
use bridge_shared::{
	bridge_contracts::{BridgeContractCounterparty, BridgeContractCounterpartyResult},
	types::{
		Amount, BridgeTransferDetails, BridgeTransferId, HashLock, HashLockPreImage,
		InitiatorAddressCounterParty, RecipientAddress, RecipientAddressCounterparty, TimeLock,
	},
};
use rand::prelude::*;
use serde_json::Value;
use std::str::FromStr;
use url::Url;

mod event_monitoring;
mod utils;

const DUMMY_ADDRESS: AccountAddress = AccountAddress::new([0; 32]);
const COUNTERPARTY_MODULE_NAME: &str = "atomic_bridge_counterparty";

enum Call {
	Lock,
	Complete,
	Abort,
	GetDetails,
}

pub struct MovementClient {
	counterparty_address: AccountAddress,
	initiator_address: Vec<u8>,
	//Added as a workaround before Address type problem is resolved
	recipient_address: AccountAddress,
	rest_client: Client,
	faucet_client: FaucetClient,
	signer: LocalAccount,
}

impl MovementClient {
	pub async fn build_with_config() -> Result<Self, anyhow::Error> {
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

		let faucet_listen_address = suzuka_config
			.execution_config
			.maptos_config
			.client
			.maptos_faucet_rest_connection_hostname;
		let faucet_listen_port = suzuka_config
			.execution_config
			.maptos_config
			.client
			.maptos_faucet_rest_connection_port;
		let faucet_url = format!("http://{}:{}", faucet_listen_address, faucet_listen_port);
		let faucet_url = Url::from_str(faucet_url.as_str()).unwrap();

		let rest_client = Client::new(node_connection_url.clone());
		let faucet_client = FaucetClient::new(faucet_url, node_connection_url.clone());

		let seed = [3u8; 32];
		let mut rng = rand::rngs::StdRng::from_seed(seed);
		let signer = LocalAccount::generate(&mut rng);

		Ok(MovementClient {
			initiator_address: Vec::new(), //dummy for now
			recipient_address: DUMMY_ADDRESS,
			rest_client,
			faucet_client,
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
	type Address = AccountAddress;
	type Hash = [u8; 32];

	async fn lock_bridge_transfer_assets(
		&mut self,
		bridge_transfer_id: BridgeTransferId<Self::Hash>,
		hash_lock: HashLock<Self::Hash>,
		time_lock: TimeLock,
		recipient: RecipientAddress,
		amount: Amount,
	) -> BridgeContractCounterpartyResult<()> {
		let payload = TransactionPayload::EntryFunction(EntryFunction {
			function: self.counterparty_function(Call::Lock),
			arguments: vec![
				Value::Array(bytes_to_json_array(self.initiator_address)),
				Value::Array(hash_to_json_array(&bridge_transfer_id.0)),
				Value::Array(hash_to_json_array(&hash_lock.0)),
				Value::Number(time_lock.0.into()),
				Value::String(self.recipient_address.to_string()),
			],
			type_arguments: self.counterparty_type_tag(Call::Lock),
		});
		let _ = utils::send_aptos_transaction(&self.rest_client, &mut self.signer, payload)
			.await
			.map_err(|e| BridgeContractCounterpartyError::generic(e));
		Ok(())
	}

	async fn complete_bridge_transfer(
		&mut self,
		bridge_transfer_id: BridgeTransferId<Self::Hash>,
		preimage: HashLockPreImage,
	) -> BridgeContractCounterpartyResult<()> {
		let payload = TransactionPayload::EntryFunctionPayload(EntryFunctionPayload {
			function: self.counterparty_function(Call::Complete),
			arguments: vec![
				Value::String(self.signer.address().to_string()),
				Value::Array(hash_to_json_array(&bridge_transfer_id.0)),
				Value::Array(self.hash_to_json_array(&preimage.0[..])),
			],
			type_arguments: self.counterparty_type_tag(Call::Complete),
		});
		let _ = utils::send_aptos_transaction(&self.rest_client, &mut self.signer, payload)?;
		Ok(())
	}

	async fn abort_bridge_transfer(
		&mut self,
		bridge_transfer_id: BridgeTransferId<Self::Hash>,
	) -> BridgeContractCounterpartyResult<()> {
		let payload = TransactionPayload::EntryFunctionPayload(EntryFunctionPayload {
			function: self.counterparty_function(Call::Abort),
			arguments: vec![Value::Array(self.hash_to_json_array(&bridge_transfer_id.0))],
			type_arguments: self.counterparty_type_tag(Call::Abort),
		});
		let _ = utils::send_aptos_transaction(&self.rest_client, &mut self.signer, payload)?;
		Ok(())
	}

	async fn get_bridge_transfer_details(
		&mut self,
		bridge_transfer_id: BridgeTransferId<Self::Hash>,
	) -> BridgeContractCounterpartyResult<Option<BridgeTransferDetails<Self::Hash, Self::Address>>>
	{
		todo!()
	}
}

impl MovementClient {
	fn counterparty_module_id(&self) -> MoveModuleId {
		MoveModuleId {
			address: self.counterparty_address.into(),
			name: IdentifierWrapper::from_str(COUNTERPARTY_MODULE_NAME).unwrap(),
		}
	}

	fn initiator_module_id(&self) -> MoveModuleId {
		todo!()
	}

	fn counterparty_type_tag(&self, call: Call) -> Vec<MoveType> {
		match call {
			Call::Lock => vec![MoveType::Address, MoveType::U64, MoveType::U64, MoveType::U8],
			Call::Complete => vec![MoveType::Address, MoveType::U64, MoveType::U8],
			Call::Abort => vec![MoveType::Address, MoveType::U64],
			Call::GetDetails => vec![MoveType::Address, MoveType::U64],
		}
	}

	fn counterparty_function(&self, call: Call) -> EntryFunctionId {
		EntryFunctionId {
			module: self.counterparty_module_id(),
			name: IdentifierWrapper::from_str(match call {
				Call::Lock => "lock_bridge_transfer_assets",
				Call::Complete => "complete_bridge_transfer",
				Call::Abort => "abort_bridge_transfer",
				Call::GetDetails => "get_bridge_transfer_details",
			}),
		}
	}

	fn move_bytes(&self) -> MoveType {
		MoveType::Vector { items: Box::new(vec![MoveType::U8(0)]) }
	}
}

fn hash_to_json_array(hash: &[u8]) -> Vec<Value> {
	hash.iter().map(|&byte| Value::Number(byte.into())).collect()
}

fn bytes_to_json_array(bytes: Vec<u8>) -> Vec<Value> {
	bytes.iter().map(|&b| Value::Number(b.into())).collect()
}
