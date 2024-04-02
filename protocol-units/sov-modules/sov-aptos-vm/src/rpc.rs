use std::array::TryFromSliceError;

use aptos_api_types::{Address, MoveModuleBytecode, MoveResource, U64};
use aptos_crypto::bls12381::Signature;
use aptos_types::state_store::state_value::StateValue as AptosStateValue;
use aptos_types::transaction::Version;
use jsonrpsee::core::RpcResult;
use reth_primitives::{TransactionSignedEcRecovered, U128};
use revm::primitives::{
	ExecutionResult, HaltReason, InvalidTransaction, TransactTo, B256, KECCAK_EMPTY, U256,
};
use sov_modules_api::macros::rpc_gen;
use sov_modules_api::{
	CryptoSpec, DaSpec, StateMap, StateMapAccessor, StateValueAccessor, StateVecAccessor,
	WorkingSet,
};
use tracing::debug;

use crate::aptos::error::rpc::EthApiError;
use crate::aptos::primitive_types::{
	BlockEnv, Receipt, SealedBlock, TransactionSignedAndRecovered,
};
use crate::experimental::SovAptosVM;

#[derive(Clone)]
pub struct EthRpcConfig<S: sov_modules_api::Spec> {
	pub min_blob_size: Option<usize>,
	pub sov_tx_signer_priv_key: <S::CryptoSpec as CryptoSpec>::PrivateKey,
	// add gas_price_oracle_config here
	pub signer: DevSigner,
}

#[rpc_gen(client, server)]
impl<S: sov_modules_api::Spec, Da: DaSpec> SovAptosVM<S, Da> {
	/// Handler for `net_version`
	#[rpc_method(name = "get_ledger_info")]
	pub fn net_version(&self, working_set: &mut WorkingSet<S>) -> RpcResult<String> {
		todo!()
	}

	/// Handler for: `healthy`
	#[rpc_method(name = "healthy")]
	pub fn chain_id(&self, working_set: &mut WorkingSet<S>) -> RpcResult<Option<U64>> {
		todo!()
	}

	/// Handler for `get_block_by_signature`
	#[rpc_method(name = "get_block_by_signature")]
	pub fn get_block_by_signature(
		&self,
		signature: Signature,
		details: Option<bool>,
		working_set: &mut WorkingSet<S>,
	) -> RpcResult<Option<reth_rpc_types::RichBlock>> {
		todo!()
	}

	/// Handler for: `get_block_by_version`
	#[rpc_method(name = "get_block_by_version")]
	pub fn get_block_by_version(
		&self,
		version: Version,
		working_set: &mut WorkingSet<S>,
	) -> RpcResult<Option<AptosStateValue>> {
		debug!(?version, "AptosVM module JSON-RPC request to `get_block_by_version`");
		todo!()
	}

	/// Handler for: `get_block_by_height`
	#[rpc_method(name = "get_block_by_height")]
	pub fn get_block_by_height(
		&self,
		block_number: Option<String>,
		details: Option<bool>,
		working_set: &mut WorkingSet<S>,
	) -> RpcResult<Option<reth_rpc_types::RichBlock>> {
		todo!()
	}

	/// Handler for: `get_resources`
	#[rpc_method(name = "get_resources")]
	pub fn get_resources(
		&self,
		address: Address,
		_block_number: Option<String>,
		working_set: &mut WorkingSet<S>,
	) -> RpcResult<Vec<MoveModuleBytecode>> {
		todo!()
	}

	/// Handler for : `get_modules`
	#[rpc_method(name = "get_modules")]
	pub fn get_modules(
		&self,
		address: Address,
		_block_number: Option<String>,
		working_set: &mut WorkingSet<S>,
	) -> RpcResult<Vec<MoveResource>> {
		todo!()
	}

	/// Handler for: `eth_getStorageAt`
	#[rpc_method(name = "eth_getStorageAt")]
	pub fn get_storage_at(
		&self,
		address: Address,
		index: U256,
		_block_number: Option<String>,
		working_set: &mut WorkingSet<S>,
	) -> RpcResult<U256> {
		todo!()
	}

	/// Handler for: `eth_getTransactionCount`
	#[rpc_method(name = "get_sequence_number")]
	pub fn get_sequence_number(
		&self,
		address: Address,
		_block_number: Option<String>,
		working_set: &mut WorkingSet<S>,
	) -> RpcResult<U64> {
		todo!()
	}

	// Handler for: `eth_getTransactionByHash`
	// TODO https://github.com/Sovereign-Labs/sovereign-sdk/issues/502
	#[rpc_method(name = "get_transaction_by_hash")]
	pub fn get_transaction_by_hash(
		&self,
		hash: B256,
		working_set: &mut WorkingSet<S>,
	) -> RpcResult<Option<reth_rpc_types::Transaction>> {
		todo!()
	}

	/// Handler for: `eth_getTransactionReceipt`
	// TODO https://github.com/Sovereign-Labs/sovereign-sdk/issues/502
	#[rpc_method(name = "eth_getTransactionReceipt")]
	pub fn get_transaction_receipt(
		&self,
		hash: B256,
		working_set: &mut WorkingSet<S>,
	) -> RpcResult<Option<reth_rpc_types::TransactionReceipt>> {
		todo!()
	}

	/// Handler for: `eth_blockNumber`
	#[rpc_method(name = "eth_blockNumber")]
	pub fn block_number(&self, working_set: &mut WorkingSet<S>) -> RpcResult<U256> {
		todo!()
	}

	fn get_sealed_block_by_number(
		&self,
		_block_number: Option<String>,
		_working_set: &mut WorkingSet<S>,
	) -> BlockEnv {
		todo!()
	}
}
