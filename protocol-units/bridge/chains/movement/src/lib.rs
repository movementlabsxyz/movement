use aptos_sdk::{
	rest_client::{Client, FaucetClient},
	types::{AccountKey, LocalAccount},
};
use aptos_types::account_address::AccountAddress;
use bridge_shared::{
	bridge_contracts::{BridgeContractCounterparty, BridgeContractCounterpartyResult},
	types::{
		Amount, BridgeTransferDetails, BridgeTransferId, HashLock, HashLockPreImage,
		RecipientAddress, TimeLock,
	},
};
use rand::prelude::*;
use std::str::FromStr;
use url::Url;

mod event_monitoring;
mod utils;

const DUMMY_ADDRESS: AccountAddress = AccountAddress::new([0; 32]);

pub struct MovementClient {
	counterparty_address: AccountAddress,
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
		let mut signer = LocalAccount::generate(&mut rng);

		Ok(MovementClient {
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
		//let pauyload =
		let response = utils::send_aptos_transaction(&self.rest_client, &mut self.signer, payload);
		todo!()
	}

	async fn complete_bridge_transfer(
		&mut self,
		bridge_transfer_id: BridgeTransferId<Self::Hash>,
		secret: HashLockPreImage,
	) -> BridgeContractCounterpartyResult<()> {
		todo!()
	}

	async fn abort_bridge_transfer(
		&mut self,
		bridge_transfer_id: BridgeTransferId<Self::Hash>,
	) -> BridgeContractCounterpartyResult<()> {
		todo!()
	}

	async fn get_bridge_transfer_details(
		&mut self,
		bridge_transfer_id: BridgeTransferId<Self::Hash>,
	) -> BridgeContractCounterpartyResult<Option<BridgeTransferDetails<Self::Hash, Self::Address>>>
	{
		todo!()
	}
}
