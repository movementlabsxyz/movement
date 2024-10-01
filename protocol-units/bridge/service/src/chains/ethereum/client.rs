use super::types::{
	AlloyProvider, AtomicBridgeCounterparty, AtomicBridgeInitiator, CounterpartyContract,
	EthAddress, InitiatorContract, WETH9Contract, WETH9,
};
use super::utils::{calculate_storage_slot, send_transaction, send_transaction_rules};
use crate::chains::bridge_contracts::BridgeContract;
use crate::chains::bridge_contracts::BridgeContractError;
use crate::chains::bridge_contracts::BridgeContractResult;
use crate::types::{
	Amount, AssetType, BridgeAddress, BridgeTransferDetails, BridgeTransferId, HashLock,
	HashLockPreImage, TimeLock,
};
use alloy::primitives::{private::serde::Deserialize, Address, FixedBytes, U256};
use alloy::providers::{Provider, ProviderBuilder, RootProvider};
use alloy::signers::k256::elliptic_curve::SecretKey;
use alloy::signers::k256::Secp256k1;
use alloy::signers::local::LocalSigner;
use alloy::{
	network::EthereumWallet,
	rlp::{RlpDecodable, RlpEncodable},
};
use alloy::{pubsub::PubSubFrontend, signers::local::PrivateKeySigner};
use alloy_rlp::Decodable;
use bridge_grpc::bridge_server::Bridge;
use bridge_grpc::{InitiateBridgeTransferRequest, InitiateBridgeTransferResponse};
use serde_with::serde_as;
use std::fmt::{self, Debug};
use tonic::{Request, Response, Status};
use url::Url;
pub const GAS_LIMIT: u128 = 10_000_000_000_000_000;
pub const RETRIES: u32 = 6;

impl fmt::Debug for AtomicBridgeInitiator::wethReturn {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		// Assuming the return type is an address, for example:
		write!(f, "{:?}", self._0)
	}
}

///Configuration for the Ethereum Bridge Client
#[serde_as]
#[derive(Clone, Debug, Deserialize)]
pub struct Config {
	pub rpc_url: Url,
	pub ws_url: Url,
	#[serde_as(as = "serde_with::DisplayFromStr")]
	pub signer_private_key: PrivateKeySigner,
	pub initiator_contract: String,
	pub counterparty_contract: String,
	pub weth_contract: String,
	pub gas_limit: u64,
}

impl Config {
	pub fn build_for_test() -> Self {
		Config {
			rpc_url: "http://localhost:8545".parse().unwrap(),
			ws_url: "ws://localhost:8545".parse().unwrap(),
			signer_private_key: PrivateKeySigner::random(),
			initiator_contract: "0x1234567890abcdef1234567890abcdef12345678".to_string(),
			counterparty_contract: "0x1234567890abcdef1234567890abcdef12345678".to_string(),
			weth_contract: "0x1234567890abcdef1234567890abcdef12345678".to_string(),
			gas_limit: 10_000_000_000,
		}
	}
}

#[derive(RlpDecodable, RlpEncodable)]
struct EthBridgeTransferDetails {
	pub amount: U256,
	pub originator: EthAddress,
	pub recipient: [u8; 32],
	pub hash_lock: [u8; 32],
	pub time_lock: U256,
	pub state: u8,
}

// We need to be able to build the client and deploy the contracts
//  therfore the `initiator_contract` and `counterparty_contract`
// should be optional, as their values will be unknown at the time of building the client.
// This is true for the integration tests.
#[allow(dead_code)]
#[derive(Clone)]
pub struct EthClient {
	rpc_provider: AlloyProvider,
	rpc_port: u16,
	ws_provider: Option<RootProvider<PubSubFrontend>>,
	initiator_contract: InitiatorContract,
	counterparty_contract: CounterpartyContract,
	weth_contract: WETH9Contract,
	config: Config,
}

impl EthClient {
	pub async fn new(config: Config) -> Result<Self, anyhow::Error> {
		let rpc_provider = ProviderBuilder::new()
			.with_recommended_fillers()
			.wallet(EthereumWallet::from(config.signer_private_key.clone()))
			.on_builtin(config.rpc_url.as_str())
			.await?;

		//load smart contract
		let initiator_contract =
			AtomicBridgeInitiator::new(config.initiator_contract.parse()?, rpc_provider.clone());
		let counterparty_contract =
			CounterpartyContract::new(config.counterparty_contract.parse()?, rpc_provider.clone());
		let weth_contract = WETH9Contract::new(config.weth_contract.parse()?, rpc_provider.clone());

		//TODO: initialise / monitoring here which should setup the ws connection

		// let ws = WsConnect::new(ws_url);
		// println!("ws {:?}", ws);
		// let ws_provider = ProviderBuilder::new().on_ws(ws).await?;
		// println!("ws_provider {:?}", ws_provider);

		Ok(EthClient {
			rpc_provider,
			rpc_port: 8545,
			ws_provider: None,
			initiator_contract,
			counterparty_contract,
			weth_contract,
			config,
		})
	}

	pub async fn initialize_initiator_contract(
		&self,
		weth: EthAddress,
		owner: EthAddress,
		timelock: TimeLock,
	) -> Result<(), anyhow::Error> {
		let call = self.initiator_contract.initialize(weth.0, owner.0, U256::from(timelock.0));
		send_transaction(call.to_owned(), &send_transaction_rules(), RETRIES, GAS_LIMIT)
			.await
			.expect("Failed to send transaction");
		Ok(())
	}

	pub async fn deposit_weth_and_approve(
		&mut self,
		_caller: Address,
		amount: U256,
	) -> Result<(), anyhow::Error> {
		let deposit_weth_signer = self.get_signer_address();
		let call = self.weth_contract.deposit().value(amount);
		send_transaction(call, &send_transaction_rules(), RETRIES, GAS_LIMIT)
			.await
			.expect("Failed to deposit eth to weth contract");

		let approve_call: alloy::contract::CallBuilder<_, &_, _> =
			self.weth_contract.approve(self.initiator_contract_address()?, amount);
		let WETH9::balanceOfReturn { _0: _balance } = self
			.weth_contract
			.balanceOf(deposit_weth_signer)
			.call()
			.await
			.expect("Failed to get balance");

		send_transaction(approve_call, &send_transaction_rules(), RETRIES, GAS_LIMIT)
			.await
			.expect("Failed to approve");
		Ok(())
	}

	pub async fn get_block_number(&self) -> Result<u64, anyhow::Error> {
		self.rpc_provider
			.get_block_number()
			.await
			.map_err(|e| anyhow::anyhow!("Failed to get block number: {}", e))
	}

	pub fn set_signer_address(&mut self, key: SecretKey<Secp256k1>) {
		self.config.signer_private_key = LocalSigner::from(key);
	}

	pub fn get_signer_address(&self) -> Address {
		self.config.signer_private_key.address()
	}

	pub fn rpc_provider(&self) -> &AlloyProvider {
		&self.rpc_provider
	}

	pub fn rpc_provider_mut(&mut self) -> &mut AlloyProvider {
		&mut self.rpc_provider
	}

	pub fn rpc_port(&self) -> u16 {
		self.rpc_port
	}

	pub fn set_initiator_contract(&mut self, contract: InitiatorContract) {
		self.initiator_contract = contract;
	}

	pub fn initiator_contract_address(&self) -> BridgeContractResult<Address> {
		self.config
			.initiator_contract
			.parse()
			.map_err(|_| BridgeContractError::ContractAddressError)
	}

	pub fn set_weth_contract(&mut self, contract: WETH9Contract) {
		self.weth_contract = contract;
	}

	pub fn weth_contract_address(&self) -> BridgeContractResult<Address> {
		self.config
			.weth_contract
			.parse()
			.map_err(|_| BridgeContractError::ContractAddressError)
	}

	pub fn counterparty_contract_address(&self) -> BridgeContractResult<Address> {
		self.config
			.counterparty_contract
			.parse()
			.map_err(|_| BridgeContractError::ContractAddressError)
	}
}

#[async_trait::async_trait]
impl BridgeContract<EthAddress> for EthClient {
	// `_initiator_address`, or in the contract, `originator` is set
	// via the `msg.sender`, which is stored in the `rpc_provider`.
	// So `initiator_address` arg is not used here.
	async fn initiate_bridge_transfer(
		&self,
		initiator_address: BridgeAddress<EthAddress>,
		recipient_address: BridgeAddress<Vec<u8>>,
		hash_lock: HashLock,
		amount: Amount, // the ETH amount
	) -> BridgeContractResult<()> {
		let contract =
			AtomicBridgeInitiator::new(self.initiator_contract_address()?, &self.rpc_provider);
		let recipient_bytes: [u8; 32] =
			recipient_address.0.try_into().expect("Recipient address must be 32 bytes");
		let call = contract
			.initiateBridgeTransfer(
				U256::from(amount.value()),
				FixedBytes(recipient_bytes),
				FixedBytes(hash_lock.0),
			)
			.value(U256::from(amount.value()))
			.from(*initiator_address.0);
		let _ = send_transaction(call, &send_transaction_rules(), RETRIES, GAS_LIMIT)
			.await
			.map_err(|e| {
				BridgeContractError::GenericError(format!("Failed to send transaction: {}", e))
			})?;
		Ok(())
	}

	async fn initiator_complete_bridge_transfer(
		&mut self,
		bridge_transfer_id: BridgeTransferId,
		pre_image: HashLockPreImage,
	) -> BridgeContractResult<()> {
		// The Alloy generated type for smart contract`pre_image` arg is `FixedBytes<32>`
		// so it must be converted to `[u8; 32]`.
		let generic_error = |desc| BridgeContractError::GenericError(String::from(desc));
		let pre_image: [u8; 32] = pre_image
			.0
			.get(0..32)
			.ok_or(generic_error("Could not get required slice from pre-image"))?
			.try_into()
			.map_err(|_| generic_error("Could not convert pre-image to [u8; 32]"))?;

		let contract =
			AtomicBridgeInitiator::new(self.initiator_contract_address()?, &self.rpc_provider);
		let call = contract
			.completeBridgeTransfer(FixedBytes(bridge_transfer_id.0), FixedBytes(pre_image));
		send_transaction(call, &send_transaction_rules(), RETRIES, GAS_LIMIT)
			.await
			.expect("Failed to send transaction");
		Ok(())
	}

	async fn counterparty_complete_bridge_transfer(
		&mut self,
		bridge_transfer_id: BridgeTransferId,
		pre_image: HashLockPreImage,
	) -> BridgeContractResult<()> {
		// The Alloy generated type for smart contract`pre_image` arg is `FixedBytes<32>`
		// so it must be converted to `[u8; 32]`.
		let generic_error = |desc| BridgeContractError::GenericError(String::from(desc));
		let pre_image: [u8; 32] = pre_image
			.0
			.get(0..32)
			.ok_or(generic_error("Could not get required slice from pre-image"))?
			.try_into()
			.map_err(|_| generic_error("Could not convert pre-image to [u8; 32]"))?;

		let contract =
			AtomicBridgeInitiator::new(self.initiator_contract_address()?, &self.rpc_provider);
		let call = contract
			.completeBridgeTransfer(FixedBytes(bridge_transfer_id.0), FixedBytes(pre_image));
		send_transaction(call, &send_transaction_rules(), RETRIES, GAS_LIMIT)
			.await
			.expect("Failed to send transaction");
		Ok(())
	}

	async fn refund_bridge_transfer(
		&mut self,
		bridge_transfer_id: BridgeTransferId,
	) -> BridgeContractResult<()> {
		let contract =
			AtomicBridgeInitiator::new(self.initiator_contract_address()?, &self.rpc_provider);
		let call = contract.refundBridgeTransfer(FixedBytes(bridge_transfer_id.0));
		send_transaction(call, &send_transaction_rules(), RETRIES, GAS_LIMIT)
			.await
			.expect("Failed to send transaction");
		Ok(())
	}

	async fn lock_bridge_transfer(
		&mut self,
		bridge_transfer_id: BridgeTransferId,
		hash_lock: HashLock,
		initiator: BridgeAddress<Vec<u8>>,
		recipient: BridgeAddress<EthAddress>,
		amount: Amount,
	) -> BridgeContractResult<()> {
		let contract = AtomicBridgeCounterparty::new(
			self.counterparty_contract_address()?,
			&self.rpc_provider,
		);
		let initiator: [u8; 32] = initiator.0.try_into().unwrap();
		let call = contract.lockBridgeTransfer(
			FixedBytes(initiator),
			FixedBytes(bridge_transfer_id.0),
			FixedBytes(hash_lock.0),
			*recipient.0,
			U256::try_from(amount.0)
				.map_err(|_| BridgeContractError::ConversionFailed("U256".to_string()))?,
		);
		send_transaction(call, &send_transaction_rules(), RETRIES, GAS_LIMIT)
			.await
			.expect("Failed to send transaction");
		Ok(())
	}

	async fn abort_bridge_transfer(
		&mut self,
		bridge_transfer_id: BridgeTransferId,
	) -> BridgeContractResult<()> {
		let contract = AtomicBridgeCounterparty::new(
			self.counterparty_contract_address()?,
			&self.rpc_provider,
		);
		let call = contract.abortBridgeTransfer(FixedBytes(bridge_transfer_id.0));
		send_transaction(call, &send_transaction_rules(), RETRIES, GAS_LIMIT)
			.await
			.expect("Failed to send transaction");
		Ok(())
	}

	async fn get_bridge_transfer_details_initiator(
		&mut self,
		bridge_transfer_id: BridgeTransferId,
	) -> BridgeContractResult<Option<BridgeTransferDetails<EthAddress>>> {
		let generic_error = |desc| BridgeContractError::GenericError(String::from(desc));

		let mapping_slot = U256::from(0); // the mapping is the zeroth slot in the contract
		let key = bridge_transfer_id.0.clone();
		let storage_slot = calculate_storage_slot(key, mapping_slot);
		let storage: U256 = self
			.rpc_provider
			.get_storage_at(self.initiator_contract_address()?, storage_slot)
			.await
			.map_err(|_| generic_error("could not find storage"))?;
		let storage_bytes = storage.to_be_bytes::<32>();

		println!("storage_bytes: {:?}", storage_bytes);
		let mut storage_slice = &storage_bytes[..];
		let eth_details = EthBridgeTransferDetails::decode(&mut storage_slice)
			.map_err(|_| generic_error("could not decode storage"))?;

		Ok(Some(BridgeTransferDetails {
			bridge_transfer_id,
			initiator_address: BridgeAddress(eth_details.originator),
			recipient_address: BridgeAddress(eth_details.recipient.to_vec()),
			hash_lock: HashLock(eth_details.hash_lock),
			//@TODO unit test these wrapping to check for any nasty side effects.
			time_lock: TimeLock(eth_details.time_lock.wrapping_to::<u64>()),
			amount: Amount(AssetType::EthAndWeth((0, eth_details.amount.wrapping_to::<u64>()))),
			state: eth_details.state,
		}))
	}

	async fn get_bridge_transfer_details_counterparty(
		&mut self,
		bridge_transfer_id: BridgeTransferId,
	) -> BridgeContractResult<Option<BridgeTransferDetails<EthAddress>>> {
		let generic_error = |desc| BridgeContractError::GenericError(String::from(desc));

		let mapping_slot = U256::from(0); // the mapping is the zeroth slot in the contract
		let key = bridge_transfer_id.0.clone();
		let storage_slot = calculate_storage_slot(key, mapping_slot);
		let storage: U256 = self
			.rpc_provider
			.get_storage_at(self.initiator_contract_address()?, storage_slot)
			.await
			.map_err(|_| generic_error("could not find storage"))?;
		let storage_bytes = storage.to_be_bytes::<32>();

		println!("storage_bytes: {:?}", storage_bytes);
		let mut storage_slice = &storage_bytes[..];
		let eth_details = EthBridgeTransferDetails::decode(&mut storage_slice)
			.map_err(|_| generic_error("could not decode storage"))?;

		Ok(Some(BridgeTransferDetails {
			bridge_transfer_id,
			initiator_address: BridgeAddress(eth_details.originator),
			recipient_address: BridgeAddress(eth_details.recipient.to_vec()),
			hash_lock: HashLock(eth_details.hash_lock),
			//@TODO unit test these wrapping to check for any nasty side effects.
			time_lock: TimeLock(eth_details.time_lock.wrapping_to::<u64>()),
			amount: Amount(AssetType::EthAndWeth((0, eth_details.amount.wrapping_to::<u64>()))),
			state: eth_details.state,
		}))
	}
}

#[cfg(test)]
fn test_wrapping_to(a: &U256, b: u64) {
	assert_eq!(a.wrapping_to::<u64>(), b);
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::time::{SystemTime, UNIX_EPOCH};

	#[test]
	fn test_wrapping_to_on_eth_details() {
		let current_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
		let eth_details = EthBridgeTransferDetails {
			amount: U256::from(10u64.pow(18)),
			originator: EthAddress([0; 20].into()),
			recipient: [0; 32],
			hash_lock: [0; 32],
			time_lock: U256::from(current_time + 84600), // 1 day
			state: 1,
		};
		test_wrapping_to(&eth_details.amount, 10u64.pow(18));
		test_wrapping_to(&eth_details.time_lock, current_time + 84600);
	}

	#[test]
	fn fuzz_test_wrapping_to_on_eth_details() {
		for _ in 0..100 {
			let current_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
			let additional_time = rand::random::<u64>();
			let random_amount = rand::random::<u64>();
			let eth_details = EthBridgeTransferDetails {
				amount: U256::from(random_amount),
				originator: EthAddress([0; 20].into()),
				recipient: [0; 32],
				hash_lock: [0; 32],
				time_lock: U256::from(current_time + additional_time),
				state: 1,
			};
			test_wrapping_to(&eth_details.amount, random_amount);
			test_wrapping_to(&eth_details.time_lock, current_time + additional_time);
		}
	}
}

// Define the service struct that wraps EthClient
pub struct EthBridgeService {
	client: EthClient,
}

impl EthBridgeService {
	pub fn new(client: EthClient) -> Self {
		Self { client }
	}
}

#[tonic::async_trait]
impl Bridge for EthBridgeService {
	async fn initiate_bridge_transfer(
		&self,
		request: Request<InitiateBridgeTransferRequest>,
	) -> Result<Response<InitiateBridgeTransferResponse>, Status> {
		// Extract the request data
		let req = request.into_inner();

		// Call the EthClient method to initiate the bridge transfer
		match <EthClient as crate::chains::bridge_contracts::BridgeContract<EthAddress>>::initiate_bridge_transfer(
            &self.client,
            BridgeAddress(EthAddress(self.client.get_signer_address())),
            BridgeAddress(req.recipient_address),
            HashLock(req.hash_lock.try_into().unwrap()),
            Amount(AssetType::Moveth(req.amount)),
        ).await {
            Ok(_) => Ok(Response::new(InitiateBridgeTransferResponse {
                success: true,
                error_message: "".to_string(),
            })),
            Err(e) => Ok(Response::new(InitiateBridgeTransferResponse {
                success: false,
                error_message: format!("Failed to initiate bridge transfer: {}", e),
            })),
        }
	}
}
