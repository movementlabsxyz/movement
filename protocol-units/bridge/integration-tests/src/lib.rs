#![allow(dead_code)]
use alloy::node_bindings::AnvilInstance;
use alloy::primitives::Address;
use alloy::primitives::FixedBytes;
use alloy::primitives::{keccak256, U256};
use alloy::providers::ProviderBuilder;
use alloy::signers::local::PrivateKeySigner;
use alloy_network::EthereumWallet;
use aptos_sdk::rest_client::aptos_api_types::Transaction as AptosTransaction;
use aptos_sdk::rest_client::{Client, FaucetClient};
use aptos_sdk::types::account_address::AccountAddress;
use aptos_sdk::types::LocalAccount;
use bridge_config::Config;
use bridge_service::chains::bridge_contracts::BridgeContractError;
use bridge_service::chains::ethereum::types::AlloyProvider;
use bridge_service::chains::ethereum::types::AtomicBridgeInitiator;
use bridge_service::chains::ethereum::types::CounterpartyContract;
use bridge_service::chains::ethereum::types::EthAddress;
use bridge_service::chains::ethereum::utils::send_transaction;
use bridge_service::chains::ethereum::utils::send_transaction_rules;
use bridge_service::chains::ethereum::{client::EthClient, types::EthHash};
use bridge_service::chains::movement::utils::{self as movement_utils, MovementAddress};
use bridge_service::chains::movement::{client::MovementClient, utils::MovementHash};
use bridge_service::types::Amount;
use bridge_service::types::BridgeTransferId;
use bridge_service::types::HashLock;
use bridge_service::types::HashLockPreImage;
use bridge_service::{chains::bridge_contracts::BridgeContractResult, types::BridgeAddress};
use godfig::{backend::config_file::ConfigFile, Godfig};
use rand::SeedableRng;
use std::str::FromStr;
use std::sync::{Arc, RwLock};
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
		Self {
			initiator: b"0x123".to_vec(),
			recipient: MovementAddress(AccountAddress::new(*b"0x00000000000000000000000000face")),
			bridge_transfer_id: MovementHash(*b"00000000000000000000000transfer1"),
			hash_lock: MovementHash(*keccak256(b"secret")),
			time_lock: 3600,
			amount: 100,
		}
	}
}

impl Default for MovementToEthCallArgs {
	fn default() -> Self {
		let preimage = "secret".to_string();
		let serialized_preimage = bcs::to_bytes(&preimage).unwrap();
		let hash_lock = *keccak256(&serialized_preimage);

		Self {
			initiator: MovementAddress(AccountAddress::new(*b"0x00000000000000000000000000face")),
			recipient: [1; 20].to_vec(),
			bridge_transfer_id: EthHash(*b"00000000000000000000000transfer1"),
			hash_lock: EthHash(hash_lock),
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

	pub async fn deposit_weth_and_approve(
		&mut self,
		initiator_address: BridgeAddress<EthAddress>,
		amount: Amount,
	) -> BridgeContractResult<()> {
		self.eth_client
			.deposit_weth_and_approve(initiator_address.0 .0, U256::from(amount.weth_value()))
			.await
			.expect("Failed to deposit WETH");
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

		let contract =
			AtomicBridgeInitiator::new(config.eth.eth_initiator_contract.parse()?, &rpc_provider);

		let initiator_address = BridgeAddress(EthAddress(initiator_address));

		let recipient_address = BridgeAddress(Into::<Vec<u8>>::into(recipient));

		let recipient_bytes: [u8; 32] =
			recipient_address.0.try_into().expect("Recipient address must be 32 bytes");
		let call = contract
			.initiateBridgeTransfer(
				U256::from(amount.weth_value()),
				FixedBytes(recipient_bytes),
				FixedBytes(hash_lock.0),
			)
			.value(U256::from(amount.eth_value()))
			.from(*initiator_address.0);
		let _ = send_transaction(
			call,
			&send_transaction_rules(),
			config.eth.transaction_send_retries,
			config.eth.gas_limit as u128,
		)
		.await
		.map_err(|e| {
			BridgeContractError::GenericError(format!("Failed to send transaction: {}", e))
		})?;
		Ok(())
	}
}

pub struct HarnessMvtClient {
	pub movement_client: MovementClient,
	///The Apotos Rest Client
	pub rest_client: Client,
	///The Apotos Rest Client
	pub faucet_client: Arc<RwLock<FaucetClient>>,
}

impl HarnessMvtClient {
	pub fn gen_aptos_account() -> Vec<u8> {
		let mut rng = ::rand::rngs::StdRng::from_seed([3u8; 32]);
		let movement_recipient = LocalAccount::generate(&mut rng);
		movement_recipient.public_key().to_bytes().to_vec()
	}

	pub async fn build(config: &Config) -> Self {
		let movement_client = MovementClient::new(&config.movement)
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

	//Mint the specified amount in MovEth.
	pub async fn mint_moveeth(
		&self,
		address: &MovementAddress,
		amount: u64,
	) -> Result<(), BridgeContractError> {
		// Mint MovETH to the initiator's address
		let mint_amount = amount; // Assuming 8 decimals for MovETH

		let mint_args = vec![
			movement_utils::serialize_address_initiator(&address.0)?, // Mint to initiator's address
			movement_utils::serialize_u64_initiator(&mint_amount)?,   // Amount to mint
		];

		let mint_payload = movement_utils::make_aptos_payload(
			self.movement_client.native_address, // Address where moveth module is published
			"moveth",
			"mint",
			Vec::new(),
			mint_args,
		);

		// Send transaction to mint MovETH
		movement_utils::send_and_confirm_aptos_transaction(
			&self.movement_client.rest_client(),
			self.movement_client.signer(),
			mint_payload,
		)
		.await
		.map_err(|_| BridgeContractError::MintError)?;
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
			self.movement_client.native_address,
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
			self.movement_client.native_address,
			bridge_service::chains::movement::client::COUNTERPARTY_MODULE_NAME,
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

		let test_mvt_hadness = HarnessMvtClient::build(&config).await;
		let test_eth_hadness = HarnessEthClient::build(&config).await;

		Ok((test_eth_hadness, test_mvt_hadness, config))
	}

	pub async fn new_with_movement(
		config: Config,
	) -> (HarnessMvtClient, Config, tokio::process::Child) {
		let (config, movement_process) = bridge_setup::test_mvt_setup(config)
			.await
			.expect("Failed to setup Movement config");

		let test_hadness = HarnessMvtClient::build(&config).await;

		(test_hadness, config, movement_process)
	}

	pub async fn new_only_eth(config: Config) -> (HarnessEthClient, Config, AnvilInstance) {
		let (config, anvil) = bridge_setup::test_eth_setup(config)
			.await
			.expect("Test eth config setup failed.");
		let test_hadness = HarnessEthClient::build(&config).await;
		(test_hadness, config, anvil)
	}
}
