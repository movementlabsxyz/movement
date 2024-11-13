use alloy::primitives::{FixedBytes, Uint, U256};
use alloy::{
	primitives::{keccak256, Address},
	providers::ProviderBuilder,
	signers::local::PrivateKeySigner,
};
use alloy_network::EthereumWallet;
use aptos_sdk::{
	rest_client::{aptos_api_types::Transaction as AptosTransaction, Client, FaucetClient},
	types::{account_address::AccountAddress, LocalAccount},
};
use bridge_config::Config;
use bridge_service::chains::ethereum::types::MockMOVEToken;
use bridge_service::chains::ethereum::types::{
	AtomicBridgeCounterpartyMOVE, AtomicBridgeInitiatorMOVE,
};
use bridge_service::chains::ethereum::utils::send_transaction;
use bridge_service::chains::ethereum::utils::send_transaction_rules;
use bridge_service::types::Amount;
use bridge_service::types::BridgeAddress;
use bridge_service::types::HashLock;
use bridge_service::{
	chains::{
		bridge_contracts::{BridgeContractError, BridgeContractResult},
		ethereum::{
			client::EthClient,
			types::{AlloyProvider, EthAddress, EthHash},
		},
		movement::{
			client_framework::{MovementClientFramework, FRAMEWORK_ADDRESS},
			utils::{self as movement_utils, MovementAddress, MovementHash},
		},
	},
	types::{BridgeTransferId, HashLockPreImage},
};
use godfig::{backend::config_file::ConfigFile, Godfig};
use rand::{distributions::Alphanumeric, thread_rng, Rng, SeedableRng};
use std::{
	convert::TryInto,
	str::FromStr,
	sync::{Arc, RwLock},
};
use url::Url;

pub mod utils;

#[derive(Clone)]
pub struct EthToMovementCallArgs {
	pub initiator: Vec<u8>,
	pub recipient: MovementAddress,
	pub bridge_transfer_id: MovementHash,
	pub hash_lock: MovementHash,
	pub time_lock: u64,
	pub amount: u64,
}

#[derive(Clone)]
pub struct MovementToEthCallArgs {
	pub initiator: MovementAddress,
	pub recipient: Vec<u8>,
	pub bridge_transfer_id: EthHash,
	pub hash_lock: EthHash,
	pub time_lock: u64,
	pub amount: u64,
}

impl Default for EthToMovementCallArgs {
	fn default() -> Self {
		// Generate 6 random alphanumeric characters
		let random_suffix: String =
			thread_rng().sample_iter(&Alphanumeric).take(6).map(char::from).collect();

		// Construct the bridge_transfer_id with the random suffix
		let mut bridge_transfer_id = b"00000000000000000000000tra".to_vec();
		bridge_transfer_id.extend_from_slice(random_suffix.as_bytes());

		Self {
			// Dummy valid EIP-55 address used in framework modules
			// initiator: b"32Be343B94f860124dC4fEe278FDCBD38C102D88".to_vec(),
			// Actual Eth address
			initiator: b"0x3c44cdddb6a900fa2b585dd299e03d12fa4293bc".to_vec(),
			// All lowercase version:
			//initiator: b"0x32be343b94f860124dc4fee278fdcbd38c102d88".to_vec(),
			// Dummy recipient address
			recipient: MovementAddress(AccountAddress::new(*b"0x00000000000000000000000000face")),
			// Convert to [u8; 32] with explicit type annotation
			bridge_transfer_id: MovementHash(
				bridge_transfer_id
					.as_slice()
					.try_into()
					.expect("Expected bridge_transfer_id to be 32 bytes"),
			),
			hash_lock: MovementHash(*keccak256(b"secret")),
			time_lock: 3600,
			amount: 100,
		}
	}
}

impl Default for MovementToEthCallArgs {
	fn default() -> Self {
		Self {
			initiator: MovementAddress(AccountAddress::new(*b"0x000000000000000000000000A55018")),
			recipient: b"32Be343B94f860124dC4fEe278FDCBD38C102D88".to_vec(),
			bridge_transfer_id: EthHash(*b"00000000000000000000000transfer1"),
			hash_lock: EthHash(*keccak256(b"secret")),
			time_lock: 3600,
			amount: 100,
		}
	}
}

pub struct HarnessEthClient {
	pub eth_rpc_url: String,
	pub signer_private_key: PrivateKeySigner,
	pub eth_client: EthClient,
}

impl HarnessEthClient {
	pub async fn build(config: &Config) -> Self {
		let eth_rpc_url = config.eth.eth_rpc_connection_url().clone();

		let signer_private_key = config
			.eth
			.signer_private_key
			.parse::<PrivateKeySigner>()
			.expect("Error during parsing signer private key?");

		let eth_client = EthClient::new(&config.eth).await.expect("Failed to create EthClient");
		HarnessEthClient { eth_client, eth_rpc_url, signer_private_key }
	}

	pub async fn rpc_provider(&self) -> AlloyProvider {
		let rpc_provider = ProviderBuilder::new()
			.with_recommended_fillers()
			.wallet(EthereumWallet::from(self.signer_private_key.clone()))
			.on_builtin(&self.eth_rpc_url)
			.await
			.expect("Error during provider creation");
		rpc_provider
	}

	pub fn signer_address(&self) -> Address {
		self.signer_private_key.address()
	}

	pub fn get_initiator_private_key(config: &Config) -> PrivateKeySigner {
		let signer_private_key = config.testing.eth_well_known_account_private_keys[2]
			.clone()
			.parse::<PrivateKeySigner>()
			.unwrap();
		signer_private_key
	}

	pub fn get_initiator_address(config: &Config) -> Address {
		HarnessEthClient::get_initiator_private_key(config).address()
	}

	pub fn get_recipient_private_key(config: &Config) -> PrivateKeySigner {
		let signer_private_key = config.testing.eth_well_known_account_private_keys[3]
			.clone()
			.parse::<PrivateKeySigner>()
			.unwrap();
		signer_private_key
	}

	pub fn get_recipeint_address(config: &Config) -> Address {
		HarnessEthClient::get_recipient_private_key(config).address()
	}

	pub async fn initiator_complete_bridge_transfer(
		&self,
		config: &Config,
		initiator_privatekey: PrivateKeySigner,
		transfer_id: BridgeTransferId,
		pre_image: HashLockPreImage,
	) -> Result<(), anyhow::Error> {
		let initiator_address = initiator_privatekey.address();
		let rpc_provider = ProviderBuilder::new()
			.with_recommended_fillers()
			.wallet(EthereumWallet::from(initiator_privatekey))
			.on_builtin(&config.eth.eth_rpc_connection_url())
			.await?;

		let contract = AtomicBridgeInitiatorMOVE::new(
			Address::from_str(&config.eth.eth_initiator_contract)?,
			rpc_provider.clone(),
		);

		let call =
			contract.completeBridgeTransfer(FixedBytes(transfer_id.0), FixedBytes(*pre_image));

		let _ = send_transaction(
			call,
			initiator_address,
			&send_transaction_rules(),
			config.eth.transaction_send_retries,
			config.eth.gas_limit as u128,
		)
		.await?;
		Ok(())
	}

	pub async fn initiate_eth_bridge_transfer(
		config: &Config,
		initiator_privatekey: PrivateKeySigner,
		recipient: MovementAddress,
		hash_lock: HashLock,
		amount: Amount,
	) -> Result<(), anyhow::Error> {
		let initiator_address = initiator_privatekey.address();
		let rpc_provider = ProviderBuilder::new()
			.with_recommended_fillers()
			.wallet(EthereumWallet::from(initiator_privatekey))
			.on_builtin(&config.eth.eth_rpc_connection_url())
			.await?;

		let mock_move_token = MockMOVEToken::new(
			Address::from_str(&config.eth.eth_move_token_contract)?,
			&rpc_provider,
		);

		// Initialize the MockMOVEToken contract with the initiator address as the initial fund recipient
		let initialize_call = mock_move_token.initialize(initiator_address).from(initiator_address);

		let _ = send_transaction(
			initialize_call,
			initiator_address,
			&send_transaction_rules(),
			config.eth.transaction_send_retries,
			config.eth.gas_limit as u128,
		)
		.await;

		// Approve the ETH initiator contract to spend Amount of MOVE
		let move_value = U256::from(amount.0.clone());
		tracing::info!("MOVE value: {}", move_value);
		let approve_call = mock_move_token
			.approve(Address::from_str(&config.eth.eth_initiator_contract)?, move_value)
			.from(initiator_address);

		let number_retry = config.eth.transaction_send_retries;
		let gas_limit = config.eth.gas_limit as u128;

		let transaction_receipt = send_transaction(
			approve_call,
			initiator_address,
			&send_transaction_rules(),
			number_retry,
			gas_limit,
		)
		.await?;

		tracing::info!("After token approval, transaction receipt: {:?}", transaction_receipt);
		let token_balance = mock_move_token.balanceOf(initiator_address).call().await?;
		tracing::info!("After token approval, MockMOVEToken balance: {:?}", token_balance._0);

		// Instantiate AtomicBridgeInitiatorMOVE
		let initiator_contract_address = config.eth.eth_initiator_contract.parse()?;
		let initiator_contract =
			AtomicBridgeInitiatorMOVE::new(initiator_contract_address, &rpc_provider);

		let recipient_address = BridgeAddress(Into::<Vec<u8>>::into(recipient));
		let recipient_bytes: [u8; 32] =
			recipient_address.0.try_into().expect("Recipient address must be 32 bytes");

		tracing::info!("Before initiate");
		tracing::info!("Amount: {}", U256::from(amount.0));
		tracing::info!("Recipient: {}", FixedBytes(recipient_bytes));
		tracing::info!("hashLock: {}", FixedBytes(hash_lock.0));

		let call = initiator_contract
			.initiateBridgeTransfer(
				U256::from(amount.0),
				FixedBytes(recipient_bytes),
				FixedBytes(hash_lock.0),
			)
			.from(initiator_address);
		let _ = send_transaction(
			call,
			initiator_address,
			&send_transaction_rules(),
			config.eth.transaction_send_retries,
			config.eth.gas_limit as u128,
		)
		.await?;

		Ok(())
	}
}

pub struct HarnessMvtClient {
	/// The Client for the Movement Framework
	pub movement_client: MovementClientFramework,
	///The Apotos Rest Client
	pub rest_client: Client,
	/// The Aptos Faucet Client
	pub faucet_client: Arc<RwLock<FaucetClient>>,
}

impl HarnessMvtClient {
	pub fn gen_aptos_account_bytes() -> Vec<u8> {
		let mut rng = ::rand::rngs::StdRng::from_seed([3u8; 32]);
		let movement_recipient = LocalAccount::generate(&mut rng);
		movement_recipient.public_key().to_bytes().to_vec()
	}
	pub fn gen_aptos_account() -> LocalAccount {
		let mut rng = ::rand::rngs::StdRng::from_seed([3u8; 32]);
		LocalAccount::generate(&mut rng)
	}

	pub async fn build(config: &Config) -> Self {
		let movement_client = MovementClientFramework::new(&config.movement)
			.await
			.expect("Failed to create MovementClient");

		let node_connection_url = Url::from_str(&config.movement.mvt_rpc_connection_url())
			.expect("Bad movement rpc url in config");
		let rest_client = Client::new(node_connection_url.clone());

		let faucet_url = Url::from_str(&config.movement.mvt_faucet_connection_url())
			.expect("Bad movement faucet url in config");
		let faucet_client = Arc::new(RwLock::new(FaucetClient::new(
			faucet_url.clone(),
			node_connection_url.clone(),
		)));

		HarnessMvtClient { movement_client, rest_client, faucet_client }
	}

	pub async fn fund_account(&self) -> LocalAccount {
		let account = LocalAccount::generate(&mut rand::rngs::OsRng);
		self.faucet_client
			.write()
			.unwrap()
			.fund(account.address(), 100_000_000)
			.await
			.expect("Failed to fund account");
		account
	}

	pub async fn init_set_timelock(&mut self, timelock: u64) -> Result<(), BridgeContractError> {
		self.movement_client.initiator_set_timelock(timelock).await?;
		Ok(())
	}

	pub async fn initiate_bridge_transfer(
		&mut self,
		initiator: &LocalAccount,
		recipient: EthAddress,
		hash_lock: HashLock,
		amount: u64,
	) -> BridgeContractResult<()> {
		let recipient_bytes: Vec<u8> = recipient.into();
		let args = vec![
			movement_utils::serialize_vec_initiator(&recipient_bytes)?,
			movement_utils::serialize_vec_initiator(&hash_lock.0[..])?,
			movement_utils::serialize_u64_initiator(&amount)?,
		];

		let payload = movement_utils::make_aptos_payload(
			FRAMEWORK_ADDRESS,
			"atomic_bridge_initiator",
			"initiate_bridge_transfer",
			Vec::new(),
			args,
		);

		let _ = movement_utils::send_and_confirm_aptos_transaction(
			&self.movement_client.rest_client,
			initiator,
			payload,
		)
		.await
		.map_err(|_| BridgeContractError::InitiateTransferError)?;

		Ok(())
	}

	pub async fn counterparty_complete_bridge_transfer(
		&mut self,
		recipient_privatekey: LocalAccount,
		bridge_transfer_id: BridgeTransferId,
		preimage: HashLockPreImage,
	) -> BridgeContractResult<AptosTransaction> {
		let unpadded_preimage = {
			let mut end = preimage.0.len();
			while end > 0 && preimage.0[end - 1] == 0 {
				end -= 1;
			}
			&preimage.0[..end]
		};
		let args2 = vec![
			bridge_service::chains::movement::utils::serialize_vec(&bridge_transfer_id.0[..])?,
			bridge_service::chains::movement::utils::serialize_vec(&unpadded_preimage)?,
		];

		let payload = bridge_service::chains::movement::utils::make_aptos_payload(
			FRAMEWORK_ADDRESS,
			bridge_service::chains::movement::client_framework::COUNTERPARTY_MODULE_NAME,
			"complete_bridge_transfer",
			Vec::new(),
			args2,
		);

		bridge_service::chains::movement::utils::send_and_confirm_aptos_transaction(
			&self.rest_client,
			&recipient_privatekey,
			payload,
		)
		.await
		.map_err(|_| BridgeContractError::CompleteTransferError)
	}
}

pub struct TestHarness;
impl TestHarness {
	pub async fn read_bridge_config() -> Result<Config, anyhow::Error> {
		let mut dot_movement = dot_movement::DotMovement::try_from_env()?;
		let pathbuff = bridge_config::get_config_path(&dot_movement);
		dot_movement.set_path(pathbuff);
		let config_file = dot_movement.try_get_or_create_config_file().await?;

		// get a matching godfig object
		let godfig: Godfig<Config, ConfigFile> = Godfig::new(ConfigFile::new(config_file), vec![]);
		let bridge_config: Config = godfig.try_wait_for_ready().await?;
		Ok(bridge_config)
	}

	pub async fn new_with_eth_and_movement(
	) -> Result<(HarnessEthClient, HarnessMvtClient, Config), anyhow::Error> {
		let config = TestHarness::read_bridge_config().await?;

		let test_mvt_harness = HarnessMvtClient::build(&config).await;
		let test_eth_harness = HarnessEthClient::build(&config).await;

		Ok((test_eth_harness, test_mvt_harness, config))
	}

	pub async fn new_with_movement() -> Result<(HarnessMvtClient, Config), anyhow::Error> {
		let config = TestHarness::read_bridge_config().await?;
		let test_harness = HarnessMvtClient::build(&config).await;

		Ok((test_harness, config))
	}

	pub async fn new_only_eth() -> Result<(HarnessEthClient, Config), anyhow::Error> {
		let config = TestHarness::read_bridge_config().await?;
		let test_harness = HarnessEthClient::build(&config).await;
		Ok((test_harness, config))
	}
}
