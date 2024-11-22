use super::types::{AlloyProvider, AssetKind, EthAddress, NativeBridge, NativeBridgeContract};
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
use bridge_util::chains::bridge_contracts::BridgeTransferInitiatedDetails;
use bridge_util::chains::bridge_contracts::{
	BridgeClientContract, BridgeContractError, BridgeContractResult, BridgeRelayerContract,
};
use bridge_util::types::Nonce;
use bridge_util::types::{Amount, BridgeAddress, BridgeTransferId};
use std::{fmt::Debug, net::SocketAddr};
use tonic::transport::Server;
use url::Url;

/// Configuration for the Ethereum Bridge Client
#[derive(Clone, Debug)]
pub struct Config {
	pub rpc_url: Url,
	pub signer_private_key: PrivateKeySigner,
	pub native_contract: Address,
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
			native_contract: conf.eth_native_contract.parse()?,
			movetoken_contract: conf.eth_move_token_contract.parse()?,
			gas_limit: conf.gas_limit.into(),
			transaction_send_retries: conf.transaction_send_retries,
			asset: conf.asset.clone().into(),
		})
	}
}

#[derive(RlpDecodable, RlpEncodable)]
struct EthBridgeTransferDetailsInitiate {
	pub amount: U256,
	pub originator: EthAddress,
	pub recipient: [u8; 32],
	pub nonce: U256,
}

#[derive(RlpDecodable, RlpEncodable)]
struct EthBridgeTransferDetailsComplete {
	pub amount: U256,
	pub originator: [u8; 32],
	pub recipient: EthAddress,
	pub nonce: U256,
}

#[derive(Clone)]
pub struct EthClient {
	pub rpc_provider: AlloyProvider,
	native_bridge_contract: NativeBridgeContract,
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

		let native_bridge_contract =
			NativeBridgeContract::new(config.native_contract, rpc_provider.clone());

		Ok(EthClient {
			rpc_provider,
			native_bridge_contract,
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

	pub async fn get_block_number(&self) -> Result<u64, anyhow::Error> {
		self.rpc_provider
			.get_block_number()
			.await
			.map_err(|e| anyhow::anyhow!("Failed to get block number: {}", e))
	}

	pub fn get_signer_address(&self) -> Address {
		self.config.signer_private_key.address()
	}

	pub fn set_initiator_contract(&mut self, contract: NativeBridgeContract) {
		self.native_bridge_contract = contract;
	}

	pub fn native_contract_address(&self) -> Address {
		self.config.native_contract
	}
}

#[async_trait::async_trait]
impl BridgeClientContract<EthAddress> for EthClient {
	async fn initiate_bridge_transfer(
		&mut self,
		recipient: BridgeAddress<Vec<u8>>,
		amount: Amount, // the ETH amount
	) -> BridgeContractResult<()> {
		let recipient_bytes: [u8; 32] = recipient.0.try_into().map_err(|e| {
			BridgeContractError::ConversionFailed(format!(
				"Failed to convert in [u8; 32] recipient: {e:?}"
			))
		})?;
		let contract = NativeBridge::new(self.config.native_contract, self.rpc_provider.clone());
		let call = contract
			.initiateBridgeTransfer(FixedBytes(recipient_bytes), U256::from(amount.0))
			.from(self.signer_address);
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

	async fn get_bridge_transfer_details(
		&mut self,
		bridge_transfer_id: BridgeTransferId,
	) -> BridgeContractResult<Option<BridgeTransferInitiatedDetails<EthAddress>>> {
		let generic_error = |desc| BridgeContractError::GenericError(String::from(desc));

		let mapping_slot = U256::from(0); // the mapping is the zeroth slot in the contract
		let key = bridge_transfer_id.0.clone();
		let storage_slot = calculate_storage_slot(key, mapping_slot);
		let storage: U256 = self
			.rpc_provider
			.get_storage_at(self.native_contract_address(), storage_slot)
			.await
			.map_err(|_| generic_error("could not find storage"))?;
		let storage_bytes = storage.to_be_bytes::<32>();

		println!("storage_bytes: {:?}", storage_bytes);
		let mut storage_slice = &storage_bytes[..];
		let eth_details = EthBridgeTransferDetailsInitiate::decode(&mut storage_slice)
			.map_err(|_| generic_error("could not decode storage"))?;

		Ok(Some(BridgeTransferInitiatedDetails {
			bridge_transfer_id,
			initiator: BridgeAddress(eth_details.originator),
			recipient: BridgeAddress(eth_details.recipient.to_vec()),
			amount: eth_details.amount.into(),
			nonce: Nonce(eth_details.nonce.wrapping_to::<u128>()),
		}))
	}
}

#[async_trait::async_trait]
impl BridgeRelayerContract<EthAddress> for EthClient {
	async fn complete_bridge_transfer(
		&mut self,
		bridge_transfer_id: BridgeTransferId,
		initiator: BridgeAddress<Vec<u8>>,
		recipient: BridgeAddress<EthAddress>,
		amount: Amount,
		nonce: Nonce,
	) -> BridgeContractResult<()> {
		let generic_error = |desc| BridgeContractError::GenericError(String::from(desc));
		let contract = NativeBridge::new(self.config.native_contract, self.rpc_provider.clone());
		let bridge_transfer_array: [u8; 32] = bridge_transfer_id
			.0
			.get(0..32) // Get the first 32 bytes
			.ok_or_else(|| generic_error("Could not get required slice from bridge_transfer_id"))? // Handle the error if slice is not available
			.try_into() // Convert slice to array
			.map_err(|e| {
				BridgeContractError::ConversionFailed(format!(
					"Failed to convert bridge_transfer_id: {e:?}"
				))
			})?;
		let initiator: [u8; 32] = initiator.0.try_into().map_err(|_| {
			BridgeContractError::ConversionFailed("initiator must be exactly 32 bytes".to_string())
		})?;
		let call = contract.completeBridgeTransfer(
			FixedBytes(bridge_transfer_id.0),
			FixedBytes(initiator),
			recipient.0 .0,
			U256::from(amount.0),
			U256::from(nonce.0),
		);
		send_transaction(
			call,
			self.signer_address,
			&send_transaction_rules(),
			self.config.transaction_send_retries,
			self.config.gas_limit,
		)
		.await
		.map_err(|e| {
			BridgeContractError::OnChainError(format!("Failed to send transaction: {}", e))
		})?;

		Ok(())
	}

	async fn get_bridge_transfer_details_with_nonce(
		&mut self,
		nonce: Nonce,
	) -> BridgeContractResult<Option<BridgeTransferInitiatedDetails<EthAddress>>> {
		todo!()
	}

	async fn is_bridge_transfer_completed(
		&mut self,
		bridge_transfer_id: BridgeTransferId,
	) -> BridgeContractResult<bool> {
		todo!()
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
		let eth_details = EthBridgeTransferDetailsInitiate {
			amount: U256::from(10u64.pow(18)),
			originator: EthAddress([0; 20].into()),
			recipient: [0; 32],
			nonce: U256::from(current_time + 84600), // 1 day
		};
		test_wrapping_to(&eth_details.amount, 10u64.pow(18));
		test_wrapping_to(&eth_details.nonce, current_time + 84600);
	}

	#[test]
	fn fuzz_test_wrapping_to_on_eth_details() {
		for _ in 0..100 {
			let current_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
			let additional_time = rand::random::<u64>();
			let random_amount = rand::random::<u64>();
			let eth_details = EthBridgeTransferDetailsInitiate {
				amount: U256::from(random_amount),
				originator: EthAddress([0; 20].into()),
				recipient: [0; 32],
				nonce: U256::from(current_time + additional_time),
			};
			test_wrapping_to(&eth_details.amount, random_amount);
			test_wrapping_to(&eth_details.nonce, current_time + additional_time);
		}
	}
}
