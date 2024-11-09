use super::types::{
	AlloyProvider, AssetKind, AtomicBridgeCounterpartyMOVE, AtomicBridgeInitiatorMOVE,
	CounterpartyContract, EthAddress, InitiatorContract,
};
use super::utils::{calculate_storage_slot, send_transaction, send_transaction_rules};
use alloy::{
	network::EthereumWallet,
	primitives::{Address, FixedBytes, U256},
	providers::{Provider, ProviderBuilder},
	rlp::{RlpDecodable, RlpEncodable},
	signers::local::PrivateKeySigner,
};
use alloy_rlp::Decodable;
use bridge_config::common::eth::EthConfig;
use bridge_grpc::bridge_server::BridgeServer;
use bridge_util::chains::bridge_contracts::{BridgeContractError, BridgeContractResult};
use bridge_util::types::{
	Amount, BridgeAddress, BridgeTransferDetails, BridgeTransferDetailsCounterparty,
	BridgeTransferId, HashLock, HashLockPreImage, TimeLock,
};
use std::{fmt::Debug, net::SocketAddr};
use tonic::transport::Server;
use tracing::info;
use url::Url;

/// Configuration for the Ethereum Bridge Client
#[derive(Clone, Debug)]
pub struct Config {
	pub rpc_url: Url,
	pub signer_private_key: PrivateKeySigner,
	pub initiator_contract: Address,
	pub counterparty_contract: Address,
	pub movetoken_contract: Address,
	pub gas_limit: u128,
	pub transaction_send_retries: u32,
	pub asset: AssetKind,
}
impl TryFrom<&EthConfig> for Config {
	type Error = anyhow::Error;

	fn try_from(conf: &EthConfig) -> Result<Self, Self::Error> {
		let signer_private_key = conf.signer_private_key.parse::<PrivateKeySigner>()?;
		let rpc_url = conf.eth_rpc_connection_url().parse()?;

		Ok(Config {
			rpc_url,
			signer_private_key,
			initiator_contract: conf.eth_initiator_contract.parse()?,
			counterparty_contract: conf.eth_counterparty_contract.parse()?,
			movetoken_contract: conf.eth_move_token_contract.parse()?,
			gas_limit: conf.gas_limit.into(),
			transaction_send_retries: conf.transaction_send_retries,
			asset: conf.asset.clone().into(),
		})
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

#[derive(RlpDecodable, RlpEncodable)]
struct EthBridgeTransferDetailsCounterparty {
	pub amount: U256,
	pub originator: [u8; 32],
	pub recipient: EthAddress,
	pub hash_lock: [u8; 32],
	pub time_lock: U256,
	pub state: u8,
}

#[derive(Clone)]
pub struct EthClient {
	pub rpc_provider: AlloyProvider,
	initiator_contract: InitiatorContract,
	counterparty_contract: CounterpartyContract,
	pub config: Config,
	signer_address: Address,
}

impl EthClient {
	pub async fn new(config: &EthConfig) -> Result<Self, anyhow::Error> {
		let config: Config = config.try_into()?;
		let signer_address = config.signer_private_key.address();
		let rpc_provider = ProviderBuilder::new()
			.with_recommended_fillers()
			.wallet(EthereumWallet::from(config.signer_private_key.clone()))
			.on_builtin(config.rpc_url.as_str())
			.await?;

		let initiator_contract =
			AtomicBridgeInitiatorMOVE::new(config.initiator_contract, rpc_provider.clone());
		let counterparty_contract =
			AtomicBridgeCounterpartyMOVE::new(config.counterparty_contract, rpc_provider.clone());

		Ok(EthClient {
			rpc_provider,
			initiator_contract,
			counterparty_contract,
			config: config.clone(),
			signer_address,
		})
	}

	/// Start the gRPC server
	/// internally this passes a cloned self `EthClient` as the service.
	pub async fn serve_grpc(
		&self,
		grpc_addr: SocketAddr,
	) -> Result<(), Box<dyn std::error::Error>> {
		tracing::info!("Starting gRPC server at: {:?}", grpc_addr);
		Server::builder()
			.add_service(BridgeServer::new(self.clone()))
			.serve(grpc_addr)
			.await?;

		Ok(())
	}

	pub async fn initialize_initiator_contract(
		&self,
		weth: EthAddress,
		owner: EthAddress,
		timelock: TimeLock,
	) -> Result<(), anyhow::Error> {
		let contract = AtomicBridgeInitiatorMOVE::new(
			self.config.initiator_contract,
			self.rpc_provider.clone(),
		);
		let call = contract.initialize(weth.0, owner.0, U256::from(timelock.0), U256::from(100));
		send_transaction(
			call.to_owned(),
			self.signer_address,
			&send_transaction_rules(),
			self.config.transaction_send_retries,
			self.config.gas_limit,
		)
		.await?;

		Ok(())
	}

	pub async fn get_block_number(&self) -> Result<u64, anyhow::Error> {
		self.rpc_provider
			.get_block_number()
			.await
			.map_err(|e| anyhow::anyhow!("Failed to get block number: {}", e))
	}

	pub fn get_signer_address(&self) -> Address {
		self.config.signer_private_key.address()
	}

	pub fn set_initiator_contract(&mut self, contract: InitiatorContract) {
		self.initiator_contract = contract;
	}

	pub fn initiator_contract_address(&self) -> Address {
		self.config.initiator_contract
	}

	pub fn counterparty_contract_address(&self) -> Address {
		self.config.counterparty_contract
	}
}

#[async_trait::async_trait]
impl bridge_util::chains::bridge_contracts::BridgeContract<EthAddress> for EthClient {
	// `_initiator_address`, or in the contract, `originator` is set
	// via the `msg.sender`, which is stored in the `rpc_provider`.
	// So `initiator_address` arg is not used here.
	async fn initiate_bridge_transfer(
		&mut self,
		initiator_address: BridgeAddress<EthAddress>,
		recipient_address: BridgeAddress<Vec<u8>>,
		hash_lock: HashLock,
		amount: Amount, // the ETH amount
	) -> BridgeContractResult<()> {
		let recipient_bytes: [u8; 32] = recipient_address.0.try_into().map_err(|e| {
			BridgeContractError::ConversionFailed(format!(
				"Failed to convert in [u8; 32] recipient_address: {e:?}"
			))
		})?;
		let contract = AtomicBridgeInitiatorMOVE::new(
			self.config.initiator_contract,
			self.rpc_provider.clone(),
		);
		let call = contract
			.initiateBridgeTransfer(
				U256::from(amount.0),
				FixedBytes(recipient_bytes),
				FixedBytes(hash_lock.0),
			)
			//			.value(U256::from(amount.0))
			.from(*initiator_address.0);
		let _ = send_transaction(
			call,
			self.signer_address,
			&send_transaction_rules(),
			self.config.transaction_send_retries,
			self.config.gas_limit,
		)
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
		info! {"Pre-image: {:?}", pre_image};
		let contract = AtomicBridgeInitiatorMOVE::new(
			self.config.initiator_contract,
			self.rpc_provider.clone(),
		);
		let call = contract
			.completeBridgeTransfer(FixedBytes(bridge_transfer_id.0), FixedBytes(pre_image));
		send_transaction(
			call,
			self.signer_address,
			&send_transaction_rules(),
			self.config.transaction_send_retries,
			self.config.gas_limit,
		)
		.await
		.map_err(|e| {
			BridgeContractError::GenericError(format!("Failed to send transaction: {}", e))
		})?;

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

		let contract = AtomicBridgeCounterpartyMOVE::new(
			self.config.counterparty_contract,
			self.rpc_provider.clone(),
		);

		let call = contract
			.completeBridgeTransfer(FixedBytes(bridge_transfer_id.0), FixedBytes(pre_image));
		send_transaction(
			call,
			self.signer_address,
			&send_transaction_rules(),
			self.config.transaction_send_retries,
			self.config.gas_limit,
		)
		.await
		.map_err(|e| {
			BridgeContractError::GenericError(format!("Failed to send transaction: {}", e))
		})?;

		Ok(())
	}

	async fn refund_bridge_transfer(
		&mut self,
		bridge_transfer_id: BridgeTransferId,
	) -> BridgeContractResult<()> {
		let contract = AtomicBridgeInitiatorMOVE::new(
			self.config.counterparty_contract,
			self.rpc_provider.clone(),
		);
		let call = contract.refundBridgeTransfer(FixedBytes(bridge_transfer_id.0));
		send_transaction(
			call,
			self.signer_address,
			&send_transaction_rules(),
			self.config.transaction_send_retries,
			self.config.gas_limit,
		)
		.await
		.map_err(|e| {
			BridgeContractError::GenericError(format!("Failed to send transaction: {}", e))
		})?;

		Ok(())
	}

	// function lockBridgeTransfer(
	//     bytes32 originator,
	//     bytes32 bridgeTransferId,
	//     bytes32 hashLock,
	//     address recipient,
	//     uint256 amount
	// ) external onlyOwner returns (bool) {
	async fn lock_bridge_transfer(
		&mut self,
		bridge_transfer_id: BridgeTransferId,
		hash_lock: HashLock,
		initiator: BridgeAddress<Vec<u8>>,
		recipient: BridgeAddress<EthAddress>,
		amount: Amount,
	) -> BridgeContractResult<()> {
		tracing::info!("Begin lockBridgeTransfer");
		let initiator: [u8; 32] = initiator.0.try_into().map_err(|_| {
			BridgeContractError::ConversionFailed("lock_bridge_transfer initiator".to_string())
		})?;
		let call = self.counterparty_contract.lockBridgeTransfer(
			FixedBytes(initiator),
			FixedBytes(bridge_transfer_id.0),
			FixedBytes(hash_lock.0),
			*recipient.0,
			U256::try_from(amount.0)
				.map_err(|_| BridgeContractError::ConversionFailed("U256".to_string()))?,
		)
		.from(self.signer_address); 

		tracing::info!("Attempting lockBridgeTransfer with sender address: {:?}", self.signer_address);
	    
		let receipt = send_transaction(
			call,
			self.signer_address,
			&send_transaction_rules(),
			self.config.transaction_send_retries,
			self.config.gas_limit,
		)
		.await
		.map_err(|e| {
			BridgeContractError::GenericError(format!("Failed to send transaction: {}", e))
		})?;

		tracing::info!("LockBridgeTransfer receipt: {:?}", receipt);

		Ok(())
	}

	async fn abort_bridge_transfer(
		&mut self,
		bridge_transfer_id: BridgeTransferId,
	) -> BridgeContractResult<()> {
		let contract = AtomicBridgeCounterpartyMOVE::new(
			self.config.counterparty_contract,
			self.rpc_provider.clone(),
		);
		let call = contract.abortBridgeTransfer(FixedBytes(bridge_transfer_id.0));
		send_transaction(
			call,
			self.signer_address,
			&send_transaction_rules(),
			self.config.transaction_send_retries,
			self.config.gas_limit,
		)
		.await
		.map_err(|e| {
			BridgeContractError::GenericError(format!("Failed to send transaction: {}", e))
		})?;
		let call = contract.abortBridgeTransfer(FixedBytes(bridge_transfer_id.0));
		send_transaction(
			call,
			self.signer_address,
			&send_transaction_rules(),
			self.config.transaction_send_retries,
			self.config.gas_limit,
		)
		.await
		.map_err(|e| {
			BridgeContractError::GenericError(format!("Failed to send transaction: {}", e))
		})?;

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
			.get_storage_at(self.initiator_contract_address(), storage_slot)
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
			time_lock: TimeLock(eth_details.time_lock.wrapping_to::<u64>()),
			amount: eth_details.amount.into(),
			state: eth_details.state,
		}))
	}

	async fn get_bridge_transfer_details_counterparty(
		&mut self,
		bridge_transfer_id: BridgeTransferId,
	) -> BridgeContractResult<Option<BridgeTransferDetailsCounterparty<EthAddress>>> {
		let generic_error = |desc| BridgeContractError::GenericError(String::from(desc));

		let mapping_slot = U256::from(0); // the mapping is the zeroth slot in the contract
		let key = bridge_transfer_id.0.clone();
		let storage_slot = calculate_storage_slot(key, mapping_slot);
		let storage: U256 = self
			.rpc_provider
			.get_storage_at(self.initiator_contract_address(), storage_slot)
			.await
			.map_err(|_| generic_error("could not find storage"))?;
		let storage_bytes = storage.to_be_bytes::<32>();

		println!("storage_bytes: {:?}", storage_bytes);
		let mut storage_slice = &storage_bytes[..];
		let eth_details = EthBridgeTransferDetailsCounterparty::decode(&mut storage_slice)
			.map_err(|_| generic_error("could not decode storage"))?;

		Ok(Some(BridgeTransferDetailsCounterparty {
			bridge_transfer_id,
			initiator_address: BridgeAddress(eth_details.originator.to_vec()),
			recipient_address: BridgeAddress(eth_details.recipient),
			hash_lock: HashLock(eth_details.hash_lock),
			time_lock: TimeLock(eth_details.time_lock.wrapping_to::<u64>()),
			amount: eth_details.amount.into(),
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
