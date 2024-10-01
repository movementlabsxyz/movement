use alloy::network::EthereumWallet;
use alloy::node_bindings::anvil::AnvilInstance;
use alloy::primitives::{keccak256, Address, FixedBytes, U256};
use alloy::providers::WalletProvider;
use alloy::providers::{ProviderBuilder, RootProvider};
use alloy::pubsub::PubSubFrontend;
use alloy::signers::local::PrivateKeySigner;
use alloy::signers::{
	k256::{elliptic_curve::SecretKey, Secp256k1},
	local::LocalSigner,
};
use aptos_sdk::types::account_address::AccountAddress;
use bridge_service::chains::bridge_contracts::BridgeContractError;
use bridge_service::chains::ethereum::client::Config;
use bridge_service::chains::ethereum::client::GAS_LIMIT;
use bridge_service::chains::ethereum::client::RETRIES;
use bridge_service::chains::ethereum::types::EthHash;
use bridge_service::chains::ethereum::types::{
	AlloyProvider, AtomicBridgeCounterparty, AtomicBridgeInitiator, EthAddress, WETH9,
};
use bridge_service::chains::ethereum::utils::{send_transaction, send_transaction_rules};
use bridge_service::chains::movement::utils::MovementAddress;
use bridge_service::chains::movement::utils::MovementHash;
use bridge_service::types::Amount;
use bridge_service::types::AssetType;
use bridge_service::types::BridgeAddress;
use bridge_service::types::HashLock;

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
			recipient: b"0x123".to_vec(),
			bridge_transfer_id: EthHash(*b"00000000000000000000000transfer1"),
			hash_lock: EthHash(hash_lock),
			time_lock: 3600,
			amount: 100,
		}
	}
}

#[derive(Clone)]
pub struct SetupEthClient {
	rpc_provider: AlloyProvider,
	rpc_port: u16,
	ws_provider: Option<RootProvider<PubSubFrontend>>,
	config: Config,
}

impl SetupEthClient {
	pub async fn setup_local_ethereum_network(config: Config) -> Result<Self, anyhow::Error> {
		let rpc_provider = ProviderBuilder::new()
			.with_recommended_fillers()
			.wallet(EthereumWallet::from(config.signer_private_key.clone()))
			.on_builtin(config.rpc_url.as_str())
			.await?;

		//TODO: initialise / monitoring here which should setup the ws connection

		// let ws = WsConnect::new(ws_url);
		// println!("ws {:?}", ws);
		// let ws_provider = ProviderBuilder::new().on_ws(ws).await?;
		// println!("ws_provider {:?}", ws_provider);

		Ok(SetupEthClient { rpc_provider, rpc_port: 8545, ws_provider: None, config })
	}

	pub fn set_eth_signer(&mut self, signer: SecretKey<Secp256k1>) -> Address {
		let wallet: &mut EthereumWallet = self.rpc_provider.wallet_mut();
		let local_signer = LocalSigner::from(signer);
		wallet.register_default_signer(local_signer.clone());
		self.config.signer_private_key = local_signer;
		self.config.signer_private_key.address()
	}

	pub fn get_signer_address(&self) -> Address {
		self.config.signer_private_key.address()
	}

	pub fn initiator_contract_address(&self) -> Result<Address, anyhow::Error> {
		let address: Address = self.config.initiator_contract.parse()?;
		Ok(address)
	}

	pub async fn initialize_initiator_contract(
		&self,
		weth: EthAddress,
		owner: EthAddress,
		timelock: u64,
	) -> Result<(), anyhow::Error> {
		let initiator_contract = AtomicBridgeInitiator::new(
			self.config.initiator_contract.parse()?,
			self.rpc_provider.clone(),
		);

		let call = initiator_contract.initialize(weth.0, owner.0, U256::from(timelock));
		send_transaction(call, &send_transaction_rules(), RETRIES, GAS_LIMIT)
			.await
			.expect("Failed to send transaction");
		Ok(())
	}

	// pub async fn deposit_weth_and_approve(
	// 	&mut self,
	// 	_caller: Address,
	// 	amount: U256,
	// ) -> Result<(), anyhow::Error> {
	// 	let deposit_weth_signer = self.get_signer_address();
	// 	let call = self.weth_contract.deposit().value(amount);
	// 	send_transaction(call, &send_transaction_rules(), RETRIES, GAS_LIMIT)
	// 		.await
	// 		.expect("Failed to deposit eth to weth contract");

	// 	let approve_call: alloy::contract::CallBuilder<_, &_, _> =
	// 		self.weth_contract.approve(self.initiator_contract_address()?, amount);
	// 	let WETH9::balanceOfReturn { _0: _balance } = self
	// 		.weth_contract
	// 		.balanceOf(deposit_weth_signer)
	// 		.call()
	// 		.await
	// 		.expect("Failed to get balance");

	// 	send_transaction(approve_call, &send_transaction_rules(), RETRIES, GAS_LIMIT)
	// 		.await
	// 		.expect("Failed to approve");
	// 	Ok(())
	// }

	pub async fn deploy_initiator_contract(&mut self) -> Address {
		let contract = AtomicBridgeInitiator::deploy(self.rpc_provider.clone())
			.await
			.expect("Failed to deploy AtomicBridgeInitiator");
		self.config.initiator_contract = contract.address().to_string();
		tracing::info!("initiator_contract address: {}", self.config.initiator_contract);
		contract.address().to_owned()
	}

	pub async fn deploy_counterpart_contract(&mut self) -> Address {
		let contract = AtomicBridgeCounterparty::deploy(self.rpc_provider.clone())
			.await
			.expect("Failed to deploy AtomicBridgeInitiator");
		self.config.counterparty_contract = contract.address().to_string();
		tracing::info!("counterparty_contract address: {}", self.config.counterparty_contract);
		contract.address().to_owned()
	}

	pub async fn deploy_weth_contract(&mut self) -> Address {
		let weth = WETH9::deploy(self.rpc_provider.clone()).await.expect("Failed to deploy WETH9");
		self.config.weth_contract = weth.address().to_string();
		tracing::info!("weth_contract address: {}", self.config.weth_contract);
		weth.address().to_owned()
	}

	pub fn get_initiator_private_key(anvil: &AnvilInstance) -> PrivateKeySigner {
		anvil.keys()[2].clone().into()
	}

	pub fn get_initiator_address(anvil: &AnvilInstance) -> Address {
		SetupEthClient::get_initiator_private_key(anvil).address()
	}

	pub fn get_recipient_private_key(anvil: &AnvilInstance) -> PrivateKeySigner {
		anvil.keys()[3].clone().into()
	}

	pub fn get_recipeint_address(anvil: &AnvilInstance) -> Address {
		SetupEthClient::get_recipient_private_key(anvil).address()
	}

	pub async fn initiate_bridge_transfer(
		&mut self,
		anvil: &AnvilInstance,
		recipient: MovementAddress,
		hash_lock: HashLock,
		amount: Amount,
	) -> Result<(), anyhow::Error> {
		let rpc_provider = ProviderBuilder::new()
			.with_recommended_fillers()
			.wallet(EthereumWallet::from(SetupEthClient::get_initiator_private_key(anvil)))
			.on_builtin(self.config.rpc_url.as_str())
			.await?;

		let contract = AtomicBridgeInitiator::new(
			self.initiator_contract_address().map_err(|err| anyhow::anyhow!(err))?,
			&rpc_provider,
		);

		let initiator_address =
			BridgeAddress(EthAddress(SetupEthClient::get_initiator_address(anvil)));

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
		let _ = send_transaction(call, &send_transaction_rules(), RETRIES, GAS_LIMIT)
			.await
			.map_err(|e| {
				BridgeContractError::GenericError(format!("Failed to send transaction: {}", e))
			})?;
		Ok(())
	}

	pub async fn deposit_weth_and_approve(
		&mut self,
		caller: PrivateKeySigner,
		amount: u64,
	) -> Result<(), anyhow::Error> {
		let rpc_provider = ProviderBuilder::new()
			.with_recommended_fillers()
			.wallet(EthereumWallet::from(caller.clone()))
			.on_builtin(self.config.rpc_url.as_str())
			.await?;
		let weth_contract = WETH9::new(self.config.weth_contract.parse()?, &rpc_provider);

		let deposit_weth_address = caller.address();
		let amount = U256::from(Amount(AssetType::EthAndWeth((0, amount))).value());
		tracing::info!("deposit amount:{amount:?}");
		let call = weth_contract.deposit().value(amount);
		send_transaction(call, &send_transaction_rules(), RETRIES, GAS_LIMIT)
			.await
			.expect("Failed to deposit eth to weth contract");

		let admin_weth_contract =
			WETH9::new(self.config.weth_contract.parse()?, &self.rpc_provider);
		let approve_call: alloy::contract::CallBuilder<_, &_, _> =
			admin_weth_contract.approve(deposit_weth_address, amount);
		let WETH9::balanceOfReturn { _0: _balance } = weth_contract
			.balanceOf(deposit_weth_address)
			.call()
			.await
			.expect("Failed to get balance");

		send_transaction(approve_call, &send_transaction_rules(), RETRIES, GAS_LIMIT)
			.await
			.expect("Failed to approve");
		Ok(())
	}
}
