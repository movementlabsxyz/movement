use alloy::primitives::U256;
use alloy::{primitives::Address, providers::ProviderBuilder, signers::local::PrivateKeySigner};
use alloy_network::EthereumWallet;
use aptos_sdk::coin_client::CoinClient;
use aptos_sdk::move_types::identifier::Identifier;
use aptos_sdk::rest_client::aptos_api_types::{self, EntryFunctionId, MoveModuleId, ViewRequest};
use aptos_sdk::{
	rest_client::{Client, FaucetClient, Response},
	types::{account_address::AccountAddress, LocalAccount},
};
use bridge_config::Config;
use bridge_service::chains::ethereum::types::MockMOVEToken;
use bridge_service::chains::ethereum::utils::send_transaction;
use bridge_service::chains::ethereum::utils::send_transaction_rules;
use bridge_service::chains::movement::client_framework::FRAMEWORK_ADDRESS;
use bridge_service::chains::{
	ethereum::{client::EthClient, types::AlloyProvider},
	movement::{client_framework::MovementClientFramework, utils::MovementAddress},
};
use bridge_service::types::Amount;
use bridge_service::types::BridgeAddress;
use bridge_service::types::BridgeTransferId;
use bridge_service::types::Nonce;
use bridge_util::chains::bridge_contracts::BridgeClientContract;
use ethabi;
use godfig::{backend::config_file::ConfigFile, Godfig};
use rand::SeedableRng;
use std::{
	str::FromStr,
	sync::{Arc, RwLock},
};
use tiny_keccak::{Hasher, Keccak};
use url::Url;

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

		let eth_client = EthClient::build_with_config(&config.eth)
			.await
			.expect("Failed to create EthClient");
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

	pub fn get_recipient_address(config: &Config) -> Address {
		HarnessEthClient::get_recipient_private_key(config).address()
	}

	pub fn calculate_bridge_transfer_id(
		initiator: AccountAddress,
		recipient: Address,
		amount: Amount,
		nonce: Nonce,
	) -> BridgeTransferId {
		let mut hasher = Keccak::v256();
		hasher.update(&initiator.as_slice());
		hasher.update(&recipient.as_slice());
		let encoded = ethabi::encode(&[ethabi::Token::Uint(ethabi::Uint::from(amount.0 as u128))]);
		hasher.update(&encoded);
		let encoded = ethabi::encode(&[ethabi::Token::Uint(ethabi::Uint::from(nonce.0))]);
		hasher.update(&encoded);
		let mut output = [0u8; 32];
		hasher.finalize(&mut output);

		BridgeTransferId(output)
	}

	pub async fn initiate_eth_bridge_transfer(
		&self,
		config: &Config,
		initiator_privatekey: PrivateKeySigner,
		recipient: MovementAddress,
		amount: Amount,
	) -> Result<(), anyhow::Error> {
		let initiator_address = initiator_privatekey.address();
		let move_value = U256::from(amount.0.clone());
		tracing::info!("initiator_address: {initiator_address}");
		tracing::info!("self.signer_address(): {}", self.signer_address());

		{
			// Move some ERC token to the initiator account
			//So that he can do the bridge transfer.
			let rpc_provider = self.rpc_provider().await;
			let mock_move_token = MockMOVEToken::new(
				Address::from_str(&config.eth.eth_move_token_contract)?,
				&rpc_provider,
			);

			// Approve the ETH initiator contract to spend Amount of MOVE
			let approve_call = mock_move_token
				.approve(self.signer_address(), move_value)
				.from(self.signer_address());

			send_transaction(
				approve_call,
				self.signer_address(),
				&send_transaction_rules(),
				config.eth.transaction_send_retries,
				config.eth.gas_limit as u128,
			)
			.await?;

			//transfer the tokens to the initiator.
			let transfer_call = mock_move_token
				.transferFrom(self.signer_address(), initiator_address, move_value)
				.from(self.signer_address());

			//			transfer_call.send().await?.get_receipt().await?;

			send_transaction(
				transfer_call,
				self.signer_address(),
				&send_transaction_rules(),
				config.eth.transaction_send_retries,
				config.eth.gas_limit as u128,
			)
			.await?;
		}

		// let initiator_rpc_provider = ProviderBuilder::new()
		// 	.with_recommended_fillers()
		// 	.wallet(EthereumWallet::from(initiator_privatekey))
		// 	.on_builtin(&config.eth.eth_rpc_connection_url())
		// 	.await?;
		let mut initiator_client =
			EthClient::build_with_signer(initiator_privatekey, &config.eth).await?;

		let mock_move_token = MockMOVEToken::new(
			Address::from_str(&config.eth.eth_move_token_contract)?,
			&initiator_client.rpc_provider,
		);

		// Approve the ETH initiator contract to spend Amount of MOVE
		let approve_call = mock_move_token
			.approve(Address::from_str(&config.eth.eth_native_contract)?, move_value)
			.from(initiator_address);

		send_transaction(
			approve_call,
			initiator_address,
			&send_transaction_rules(),
			config.eth.transaction_send_retries,
			config.eth.gas_limit as u128,
		)
		.await?;

		//Initiate transfer
		let recipient_address = BridgeAddress(Into::<Vec<u8>>::into(recipient));
		initiator_client.initiate_bridge_transfer(recipient_address, amount).await?;

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

	pub fn signer_address(&self) -> AccountAddress {
		self.movement_client.signer().address()
	}

	pub async fn build(config: &Config) -> Self {
		let movement_client = MovementClientFramework::build_with_config(&config.movement)
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

	fn normalize_to_32_bytes(value: u64) -> Vec<u8> {
		// Convert the u64 value to a u256 (as bytes)
		let bytes = ethabi::encode(&[ethabi::Token::Uint(ethabi::Uint::from(value as u128))]);

		bytes
	}

	pub async fn get_bridge_fee(&self) -> Result<u64, anyhow::Error> {
		// Create the view request to call the bridge_fee Move function
		let view_request = ViewRequest {
			function: EntryFunctionId {
				module: MoveModuleId {
					address: FRAMEWORK_ADDRESS.clone().into(),
					name: aptos_api_types::IdentifierWrapper(
						Identifier::new("native_bridge_configuration").map_err(|_| {
							anyhow::anyhow!("Failed to create module name identifier")
						})?,
					),
				},
				name: aptos_api_types::IdentifierWrapper(
					Identifier::new("bridge_fee").map_err(|_| {
						anyhow::anyhow!("Failed to create function name identifier")
					})?,
				),
			},
			type_arguments: vec![],
			arguments: vec![],
		};

		// Make the view call
		let response: Response<Vec<serde_json::Value>> = self
			.rest_client
			.view(&view_request, None)
			.await
			.map_err(|err| anyhow::anyhow!("Failed to call view function: {:?}", err))?;

		let values = response.inner();

		tracing::info!("Raw response: {:?}", values);

		// Ensure the response contains exactly one value
		if values.len() != 1 {
			return Err(anyhow::anyhow!("Unexpected response length: {}", values.len()));
		}

		// Parse the bridge fee from the string
		let fee_str = values[0]
			.as_str()
			.ok_or_else(|| anyhow::anyhow!("Bridge fee is not a string"))?;
		let fee = fee_str
			.parse::<u64>()
			.map_err(|err| anyhow::anyhow!("Failed to parse bridge fee as u64: {:?}", err))?;

		Ok(fee)
	}

	pub fn calculate_bridge_transfer_id(
		initiator: Address,
		recipient: AccountAddress,
		amount: Amount,
		nonce: Nonce,
	) -> BridgeTransferId {
		let mut hasher = Keccak::v256();
		hasher.update(&initiator.as_slice());
		hasher.update(&bcs::to_bytes(&recipient).unwrap());
		let encoded_amount = Self::normalize_to_32_bytes(amount.0);
		hasher.update(&encoded_amount);
		let encoded_nonce = Self::normalize_to_32_bytes(nonce.0 as u64);
		hasher.update(&encoded_nonce);
		let mut output = [0u8; 32];
		hasher.finalize(&mut output);

		BridgeTransferId(output)
	}

	pub async fn initiate_bridge_transfer_helper_framework(
		config: &Config,
		initiator_privatekey: LocalAccount,
		recipient: Vec<u8>,
		amount: u64,
	) -> Result<(), anyhow::Error> {
		//Create a client with Initiator as signer.
		let mut movement_client =
			MovementClientFramework::build_with_signer(initiator_privatekey, &config.movement)
				.await?;

		movement_client
			.initiate_bridge_transfer(BridgeAddress(recipient), Amount(amount))
			.await?;

		Ok(())
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

	pub async fn fund_signer_and_check_balance_framework(
		&mut self,
		expected_balance: u64,
	) -> Result<(), anyhow::Error> {
		let coin_client = CoinClient::new(&self.rest_client);
		self.faucet_client
			.write()
			.unwrap()
			.fund(self.signer_address(), expected_balance)
			.await?;

		let balance = coin_client.get_account_balance(&self.signer_address()).await?;
		assert!(
			balance >= expected_balance,
			"Expected Movement Client to have at least {}, but found {}",
			expected_balance,
			balance
		);

		Ok(())
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

	// Get a different nonce for every test
	pub fn create_nonce() -> Nonce {
		let start = std::time::SystemTime::now();
		let duration_since_epoch =
			start.duration_since(std::time::UNIX_EPOCH).expect("Time went backwards");
		let timestamp_seconds = duration_since_epoch.as_millis();
		Nonce(timestamp_seconds)
	}
}
