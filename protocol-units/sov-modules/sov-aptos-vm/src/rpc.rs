use crate::DevSigner;
use std::array::TryFromSliceError;
use crate::experimental::SovAptosVM;
use aptos_api_types::{Address, MoveModuleBytecode, MoveResource, U64};
use aptos_crypto::bls12381::Signature;
use aptos_types::state_store::state_value::StateValue as AptosStateValue;
use aptos_types::transaction::Version;
use jsonrpsee::core::RpcResult;
use jsonrpsee::proc_macros::rpc;
use reth_primitives::{TransactionSignedEcRecovered, U128};
use sov_modules_api::macros::rpc_gen;
use sov_modules_api::{
	CryptoSpec, DaSpec, StateMap, StateMapAccessor, StateValueAccessor, StateVecAccessor,
	WorkingSet,
};
use tracing::debug;
use aptos_api::runtime::get_apis;
use aptos_api_types::{
    Address, EncodeSubmissionRequest, IdentifierWrapper, MoveStructTag, RawTableItemRequest,
    StateKeyWrapper, TableItemRequest, ViewRequest, U64,
};
use crate::util::sync;
use rest_to_json_rpc::JsonRpcRequest;

#[derive(Clone)]
pub struct EthRpcConfig<S: sov_modules_api::Spec> {

}

#[rpc_gen(client, server)]
impl<S: sov_modules_api::Spec> SovAptosVM<S> {

	// ACCOUNTS

	/// https://github.com/aptos-labs/aptos-core/blob/fec2fbe817df70e9c8ccb55fec52a568ec8586c5/api/src/accounts.rs#L50
	#[rpc_method(name = "accounts.address")]
	pub fn accounts_by_address(
		&self, 
		request : JsonRpcRequest,
		working_set: &mut WorkingSet<S>
	) -> RpcResult<Account> {

		// PARAMS
		let standard_request = request.try_standard()?;

		// get the accept type from the body
		let accept_type_value = standard_request.body.get("accept_type").ok_or_else(|| anyhow!("accept_type not found"))?;
		let accept_type = AcceptType::from_str(accept_type_value)?;

		// get the address from the path params
		let address_string = standard_request.path_params.get("address").ok_or_else(|| anyhow!("address not found"))?;
		let address = Address::from_str(address_string)?;

		// get the option of the ledger version from the query params
		let ledger_version : Option<u64> = standard_request.query_params.get("ledger_version").map(|v| v.parse::<Version>()).transpose()?;

		// API CONTEXT
		let aptos_api_context = self.get_aptos_api_context(working_set)?;
		let aptos_api_service  = get_apis(aptos_api_context);
		let accounts_api = aptos_api_service.api.0;

		// LOGIC
		let account_data = sync(|| async move {
			accounts_api.get_account(accept_type, address, ledger_version).await
		})?;

		// RETURN
		Ok(account_data)
	}

	/// https://github.com/aptos-labs/aptos-core/blob/fec2fbe817df70e9c8ccb55fec52a568ec8586c5/api/src/accounts.rs#L78
	#[rpc_method(name = "accounts.address.resources")]
	pub fn account_resources_by_address(
		&self,
		signature: Signature,
		details: Option<bool>,
		working_set: &mut WorkingSet<S>,
	) -> RpcResult<Option<reth_rpc_types::RichBlock>> {
		todo!()
	}

	/// Handler for: /accounts/{address}/resources
	#[rpc_method(name = "accounts.address.modules")]
	pub fn account_modules_by_address(
		&self,
		version: Version,
		working_set: &mut WorkingSet<S>,
	) -> RpcResult<Option<AptosStateValue>> {
		debug!(?version, "AptosVM module JSON-RPC request to `get_block_by_version`");
		todo!()
	}

	/// Handler for: `get_block_by_height`
	#[rpc_method(name = "accounts.address.resource.resource_type")]
	pub fn account_resource_by_address(
		&self,
		block_number: Option<String>,
		details: Option<bool>,
		working_set: &mut WorkingSet<S>,
	) -> RpcResult<Option<reth_rpc_types::RichBlock>> {
		todo!()
	}

	/// Handler for: `get_resources`
	#[rpc_method(name = "accounts.address.module.module_name")]
	pub fn get_resources(
		&self,
		address: Address,
		_block_number: Option<String>,
		working_set: &mut WorkingSet<S>,
	) -> RpcResult<Vec<MoveModuleBytecode>> {
		todo!()
	}

	// BLOCKS

	/// Handler for : `get_modules`
	#[rpc_method(name = "blocks.by_height.block_height")]
	pub fn blocks_by_block_height(
		&self,
		address: Address,
		_block_number: Option<String>,
		working_set: &mut WorkingSet<S>,
	) -> RpcResult<Vec<MoveResource>> {
		todo!()
	}

	/// Handler for: `eth_getStorageAt`
	#[rpc_method(name = "blocks.by_version.version")]
	pub fn block_by_version(
		&self,
		address: Address,
		index: U256,
		_block_number: Option<String>,
		working_set: &mut WorkingSet<S>,
	) -> RpcResult<U256> {
		todo!()
	}

	// EVENTS

	#[rpc_method(name = "accounts.address.events.creation_number")]
	pub fn account_events_by_creation_number(
		&self,
		address: Address,
		creation_number: U64,
		working_set: &mut WorkingSet<S>,
	) -> RpcResult<Vec<Receipt>> {
		todo!()
	}

	#[rpc_method(name = "accounts.address.events.event_handle.field_name")]
	pub fn account_events_by_event_handle_field_name(
		&self,
		address: Address,
		event_handle: U64,
		field_name: String,
		working_set: &mut WorkingSet<S>,
	) -> RpcResult<Vec<Receipt>> {
		todo!()
	}

	// GENERAL
	

	// TABLES

	/// Handler for: `eth_getTransactionCount`
	#[rpc_method(name = "tables.table_handle.item")]
	pub fn tables_by_table_handle(
		&self,
		address: Address,
		_block_number: Option<String>,
		working_set: &mut WorkingSet<S>,
	) -> RpcResult<U64> {
		todo!()
	}

	// Handler for: `eth_getTransactionByHash`
	// TODO https://github.com/Sovereign-Labs/sovereign-sdk/issues/502
	#[rpc_method(name = "tables.table_handle.raw_item")]
	pub fn tables_by_table_handle_raw(
		&self,
		hash: B256,
		working_set: &mut WorkingSet<S>,
	) -> RpcResult<Option<reth_rpc_types::Transaction>> {
		todo!()
	}

	/// Handler for: `eth_getTransactionReceipt`
	// TODO https://github.com/Sovereign-Labs/sovereign-sdk/issues/502
	#[rpc_method(name = "transactions")]
	pub fn transactions(
		&self,
		hash: B256,
		working_set: &mut WorkingSet<S>,
	) -> RpcResult<Option<reth_rpc_types::TransactionReceipt>> {
		todo!()
	}

	/// Handler for: `eth_blockNumber`
	#[rpc_method(name = "transactions.by_hash.txn_hash")]
	pub fn transactions_by_transaction_hash(&self, working_set: &mut WorkingSet<S>) -> RpcResult<U256> {
		todo!()
	}


	#[rpc_method(name = "transactions.by_version.txn_version")]
	pub fn transactions_by_version(
		&self,
		version: Version,
		working_set: &mut WorkingSet<S>,
	) -> RpcResult<Option<reth_rpc_types::Transaction>> {
		todo!()
	}

	#[rpc_method(name = "accounts.address.transactions")]
	pub fn account_transactions_by_address(
		&self,
		address: Address,
		working_set: &mut WorkingSet<S>,
	) -> RpcResult<Vec<reth_rpc_types::Transaction>> {
		todo!()
	}

	#[rpc_method(name = "transactions.batch")]
	pub fn transactions_batch(
		&self,
		transactions: Vec<TransactionSignedEcRecovered>,
		working_set: &mut WorkingSet<S>,
	) -> RpcResult<Vec<ExecutionResult>> {
		todo!()
	}

	#[rpc_method(name = "transactions.simulate")]
	pub fn transactions_simulate(
		&self,
		transactions: Vec<TransactionSignedEcRecovered>,
		working_set: &mut WorkingSet<S>,
	) -> RpcResult<Vec<ExecutionResult>> {
		todo!()
	}

	#[rpc_method(name = "transactions.encode_submission")]
	pub fn transactions_encode_submission(
		&self,
		transactions: Vec<TransactionSignedEcRecovered>,
		working_set: &mut WorkingSet<S>,
	) -> RpcResult<Vec<ExecutionResult>> {
		todo!()
	}

	#[rpc_method(name = "estimate_gas_price")]
	pub fn estimate_gas_price(&self, working_set: &mut WorkingSet<S>) -> RpcResult<U128> {
		todo!()
	}


	#[rpc_method(name = "view")]
	pub fn view(&self, working_set: &mut WorkingSet<S>) -> RpcResult<U128> {
		todo!()
	}
	
}
