use alloy::primitives::U256;
use alloy::{primitives::Address, providers::ProviderBuilder, signers::local::PrivateKeySigner};
use alloy_network::EthereumWallet;
use aptos_sdk::coin_client::CoinClient;
use aptos_sdk::{
	rest_client::{Client, FaucetClient},
	types::{account_address::AccountAddress, LocalAccount},
};
use bridge_config::Config;
use bridge_service::chains::ethereum::types::MockMOVEToken;
use bridge_service::chains::ethereum::utils::send_transaction;
use bridge_service::chains::ethereum::utils::send_transaction_rules;
use bridge_service::chains::{
	ethereum::{client::EthClient, types::AlloyProvider},
	movement::{client_framework::MovementClientFramework, utils::MovementAddress},
};
use bridge_service::types::Amount;
use bridge_service::types::BridgeAddress;
use bridge_service::types::BridgeTransferId;
use bridge_service::types::Nonce;
use bridge_util::chains::bridge_contracts::BridgeClientContract;
use godfig::{backend::config_file::ConfigFile, Godfig};
use rand::{distributions::Alphanumeric, thread_rng, Rng, SeedableRng};
use std::{
	convert::TryInto,
	str::FromStr,
	sync::{Arc, RwLock},
};
use tiny_keccak::{Hasher, Keccak};
use url::Url;

#[derive(Clone)]
pub struct EthToMovementCallArgs {
	pub initiator: Vec<u8>,
	pub recipient: MovementAddress,
	pub bridge_transfer_id: BridgeTransferId,
	pub amount: u64,
}

#[derive(Clone)]
pub struct MovementToEthCallArgs {
	pub initiator: MovementAddress,
	pub recipient: Vec<u8>,
	pub bridge_transfer_id: BridgeTransferId,
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
			bridge_transfer_id: BridgeTransferId(
				bridge_transfer_id
					.as_slice()
					.try_into()
					.expect("Expected bridge_transfer_id to be 32 bytes"),
			),
			amount: 100,
		}
	}
}

impl Default for MovementToEthCallArgs {
	fn default() -> Self {
		// Generate a 6-character random alphanumeric suffix
		let random_suffix: String =
			thread_rng().sample_iter(&Alphanumeric).take(6).map(char::from).collect();

		// Construct the bridge_transfer_id with the random suffix
		let mut bridge_transfer_id = b"00000000000000000000000tra".to_vec();
		bridge_transfer_id.extend_from_slice(random_suffix.as_bytes());

		Self {
			initiator: MovementAddress(AccountAddress::new(*b"0x000000000000000000000000A55018")),
			recipient: b"32Be343B94f860124dC4fEe278FDCBD38C102D88".to_vec(),
			bridge_transfer_id: BridgeTransferId(
				bridge_transfer_id
					.as_slice()
					.try_into()
					.expect("Expected bridge_transfer_id to be 32 bytes"),
			),
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

	pub fn get_initiator(config: &Config) -> Address {
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

	pub fn calculated_transfer_bridfe_id(
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
