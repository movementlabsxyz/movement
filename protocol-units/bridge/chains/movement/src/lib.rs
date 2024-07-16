use aptos_sdk::{
	move_types::{
		identifier::Identifier,
		language_storage::{ModuleId, TypeTag},
	},
	rest_client::{aptos_api_types::MoveModuleId, Client, FaucetClient},
	types::LocalAccount,
};
use aptos_types::{
	account_address::AccountAddress,
	transaction::{EntryFunction, TransactionPayload},
};
use bridge_shared::{
	bridge_contracts::{
		BridgeContractCounterparty, BridgeContractCounterpartyError,
		BridgeContractCounterpartyResult,
	},
	types::{
		Amount, BridgeTransferDetails, BridgeTransferId, HashLock, HashLockPreImage,
		RecipientAddress, TimeLock,
	},
};
use rand::prelude::*;
use serde::Serialize;
use std::str::FromStr;
use url::Url;

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
		let fn_id = self
			.counterparty_function(Call::Lock)
			.map_err(|_| BridgeContractCounterpartyError::LockTransferAssetsError)?;

		//@TODO properly return an error instead of unwrapping
		let args = vec![
			self.to_bcs_bytes(&self.signer.address()).unwrap(),
			self.to_bcs_bytes(&bridge_transfer_id.0).unwrap(),
			self.to_bcs_bytes(&hash_lock.0).unwrap(),
			self.to_bcs_bytes(&time_lock.0).unwrap(),
			self.to_bcs_bytes(&recipient.0).unwrap(),
			self.to_bcs_bytes(&amount.0).unwrap(),
		];
		let payload = TransactionPayload::EntryFunction(EntryFunction::new(
			self.counterparty_module_id(),
			fn_id,
			self.counterparty_type_args(Call::Lock),
			args,
		));
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
		let fn_id = self
			.counterparty_function(Call::Complete)
			.map_err(|_| BridgeContractCounterpartyError::CompleteTransferError)?;
		let args = vec![
			self.to_bcs_bytes(&self.signer.address()).unwrap(),
			self.to_bcs_bytes(&bridge_transfer_id.0).unwrap(),
			self.to_bcs_bytes(&preimage.0).unwrap(),
		];
		let payload = TransactionPayload::EntryFunction(EntryFunction::new(
			self.counterparty_module_id(),
			fn_id,
			self.counterparty_type_args(Call::Complete),
			args,
		));
		let _ = utils::send_aptos_transaction(&self.rest_client, &mut self.signer, payload)
			.await
			.map_err(|_| BridgeContractCounterpartyError::CompleteTransferError);
		Ok(())
	}

	async fn abort_bridge_transfer(
		&mut self,
		bridge_transfer_id: BridgeTransferId<Self::Hash>,
	) -> BridgeContractCounterpartyResult<()> {
		let fn_id = self
			.counterparty_function(Call::Abort)
			.map_err(|_| BridgeContractCounterpartyError::AbortTransferError)?;
		let args = vec![
			self.to_bcs_bytes(&self.signer.address()).unwrap(),
			self.to_bcs_bytes(&bridge_transfer_id.0).unwrap(),
		];
		let payload = TransactionPayload::EntryFunction(EntryFunction::new(
			self.counterparty_module_id(),
			fn_id,
			self.counterparty_type_args(Call::Abort),
			args,
		));
		let _ = utils::send_aptos_transaction(&self.rest_client, &mut self.signer, payload)
			.await
			.map_err(|_| BridgeContractCounterpartyError::AbortTransferError);
		Ok(())
	}

	async fn get_bridge_transfer_details(
		&mut self,
		bridge_transfer_id: BridgeTransferId<Self::Hash>,
	) -> BridgeContractCounterpartyResult<Option<BridgeTransferDetails<Self::Hash, Self::Address>>>
	{
				let _ = utils::send_view_request(self.rest_client, )
				todo!();
	}
}

impl MovementClient {
	fn counterparty_module_id(&self) -> ModuleId {
		ModuleId {
			address: self.counterparty_address.into(),
			name: Identifier::from_str(COUNTERPARTY_MODULE_NAME).unwrap(),
		}
	}

	fn initiator_module_id(&self) -> MoveModuleId {
		todo!()
	}

	fn counterparty_type_args(&self, call: Call) -> Vec<TypeTag> {
		match call {
			Call::Lock => vec![TypeTag::Address, TypeTag::U64, TypeTag::U64, TypeTag::U8],
			Call::Complete => vec![TypeTag::Address, TypeTag::U64, TypeTag::U8],
			Call::Abort => vec![TypeTag::Address, TypeTag::U64],
			Call::GetDetails => vec![TypeTag::Address, TypeTag::U64],
		}
	}

	fn counterparty_function(&self, call: Call) -> Result<Identifier, anyhow::Error> {
		let str = match call {
			Call::Lock => "lock_bridge_transfer_assets",
			Call::Complete => "complete_bridge_transfer",
			Call::Abort => "abort_bridge_transfer",
			Call::GetDetails => "get_bridge_transfer_details",
		};
		let id = Identifier::new(str)?;
		Ok(id)
	}

	fn to_bcs_bytes<T>(&self, value: &T) -> Result<Vec<u8>, anyhow::Error>
	where
		T: Serialize,
	{
		Ok(bcs::to_bytes(value)?)
	}
}
