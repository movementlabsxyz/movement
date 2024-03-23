use std::array::TryFromSliceError;

use jsonrpsee::core::RpcResult;
use reth_primitives::{TransactionSignedEcRecovered, U128};
use revm::primitives::{
	ExecutionResult, HaltReason, InvalidTransaction, TransactTo, B256, KECCAK_EMPTY, U256,
};
use sov_modules_api::macros::rpc_gen;
use sov_modules_api::{DaSpec, StateMapAccessor, StateValueAccessor, StateVecAccessor, WorkingSet};
use tracing::debug;

use aptos_api_types::{Address, MoveModuleBytecode, MoveResource, U64};
use aptos_crypto::bls12381::Signature;

use crate::aptos::db::AptosDb;
use crate::aptos::error::rpc::EthApiError;
use crate::aptos::error::rpc::RpcInvalidTransactionError;
use crate::aptos::primitive_types::{BlockEnv, Receipt, SealedBlock, TransactionSignedAndRecovered};
use crate::experimental::AptosVM;

#[rpc_gen(client, server)]
impl<S: sov_modules_api::Spec, Da: DaSpec> AptosVM<S, Da> {
	/// Handler for `net_version`
	#[rpc_method(name = "get_ledger_info")]
	pub fn net_version(&self, working_set: &mut WorkingSet<S>) -> RpcResult<String> {
		debug!("Aptos VM module JSON-RPC request to `get_ledger_info`");

		// Network ID is the same as chain ID for most networks
		// Not sure if this is the same for Aptos, unit test this.
		let chain_id = self
			.cfg
			.get(working_set)
			.expect("AptosVM config must be set at genesis")
			.chain_id;

		Ok(chain_id.to_string())
	}

	/// Handler for: `healthy`
	#[rpc_method(name = "healthy")]
	pub fn chain_id(&self, working_set: &mut WorkingSet<S>) -> RpcResult<Option<U64>> {
		let chain_id = self
			.cfg
			.get(working_set)
			.expect("AptosVM config must be set at genesis")
			.chain_id;
		debug!(chain_id = chain_id, "AptosVM module JSON-RPC request to `healthy`");
		Ok(Some(U64::from(chain_id)))
	}

	/// Handler for `get_block_by_signature`
	#[rpc_method(name = "get_block_by_signature")]
	pub fn get_block_by_signature(
		&self,
		signature: Signature,
		details: Option<bool>,
		working_set: &mut WorkingSet<S>,
	) -> RpcResult<Option<reth_rpc_types::RichBlock>> {
		debug!(?signature, "AptosVM module JSON-RPC request to `get_block_by_signature`");

		let block_number_hex = self
			.block_hashes
			.get(&signature, &mut working_set.accessory_state())
			.map(|number| hex::encode(number.to_be_bytes()))
			.expect("Block number for known block hash must be set");

		self.get_block_by_height(Some(block_number_hex), details, working_set)
	}

	/// Handler for: `get_block_by_height`
	#[rpc_method(name = "get_block_by_height")]
	pub fn get_block_by_height(
		&self,
		block_number: Option<String>,
		details: Option<bool>,
		working_set: &mut WorkingSet<S>,
	) -> RpcResult<Option<reth_rpc_types::RichBlock>> {
		debug!(block_number, "AptosVM module JSON-RPC request to `get_block_by_height`");

		let block = self.get_sealed_block_by_number(block_number, working_set);

		// Build rpc header response
		//let header = from_primitive_with_hash(block.header.clone());

		// let payload = block.payload().expect("No payload in block");
		// let transactions = match payload {
		//     Payload::DirectMempool(txs) => txs,
		//     _ => panic!("Only DirectMempool payload is supported"), // add proper error
		// };
		//
		// let block = SovAptosBlock {
		//     block,
		//     transactions: BlockTransactions::Full(transactions),
		// };
		//
		// Ok(Some(block.into()))
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
		// TODO: Implement block_number once we have archival state #951
		// https://github.com/Sovereign-Labs/sovereign-sdk/issues/951

		let resources = self
			.accounts
			.get(&address, working_set)
			.map(|account| account.info.resources)
			.unwrap_or_default();
		debug!(
			%address,
			"AptosVM module JSON-RPC request to `get_resources`"
		);

		Ok(resources)
	}

	/// Handler for : `get_modules`
	#[rpc_method(name = "get_modules")]
	pub fn get_modules(
		&self,
		address: Address,
		_block_number: Option<String>,
		working_set: &mut WorkingSet<S>,
	) -> RpcResult<Vec<MoveResource>> {
		let modules = self
			.accounts
			.get(&address, working_set)
			.map(|account| account.info.modules)
			.unwrap_or_default();

		Ok(modules)
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
		debug!("aptos module JSON-RPC request to `eth_getStorageAt`");

		// TODO: Implement block_number once we have archival state #951
		// https://github.com/Sovereign-Labs/sovereign-sdk/issues/951

		let storage_slot = self
			.accounts
			.get(&address, working_set)
			.and_then(|account| account.storage.get(&index, working_set))
			.unwrap_or_default();

		Ok(storage_slot)
	}

	/// Handler for: `eth_getTransactionCount`
	#[rpc_method(name = "get_sequence_number")]
	pub fn get_sequence_number(
		&self,
		address: Address,
		_block_number: Option<String>,
		working_set: &mut WorkingSet<S>,
	) -> RpcResult<U64> {
		// TODO: Implement block_number once we have archival state #882
		// https://github.com/Sovereign-Labs/sovereign-sdk/issues/882

		let seq_number = self
			.accounts
			.get(&address, working_set)
			.map(|account| account.info.sequence_number)
			.unwrap_or_default();

		debug!(%address, "Aptos module JSON-RPC request to `get_sequence_number`");

		Ok(seq_number)
	}

	// Handler for: `eth_getTransactionByHash`
	// TODO https://github.com/Sovereign-Labs/sovereign-sdk/issues/502
	#[rpc_method(name = "get_transaction_by_hash")]
	pub fn get_transaction_by_hash(
		&self,
		hash: B256,
		working_set: &mut WorkingSet<S>,
	) -> RpcResult<Option<reth_rpc_types::Transaction>> {
		let mut accessory_state = working_set.accessory_state();

		let tx_number = self.transaction_hashes.get(&hash, &mut accessory_state);

		let transaction = tx_number.map(|number| {
			let tx =
				self.transactions.get(number as usize, &mut accessory_state).unwrap_or_else(|| {
					panic!("Transaction with known hash {} and number {} must be set in all {} transaction",
                                          hash,
                                          number,
                                          self.transactions.len(&mut accessory_state))
				});

			let block = self
				.blocks
				.get(tx.block_number as usize, &mut accessory_state)
				.unwrap_or_else(|| {
					panic!(
						"Block with number {} for known transaction {} must be set",
						tx.block_number, tx.signed_transaction.hash
					)
				});
		});

		debug!(
			%hash,
			?transaction,
			"aptos module JSON-RPC request to `eth_getTransactionByHash`"
		);

		todo!()
		//Ok(transaction)
	}

	/// Handler for: `eth_getTransactionReceipt`
	// TODO https://github.com/Sovereign-Labs/sovereign-sdk/issues/502
	#[rpc_method(name = "eth_getTransactionReceipt")]
	pub fn get_transaction_receipt(
		&self,
		hash: B256,
		working_set: &mut WorkingSet<S>,
	) -> RpcResult<Option<reth_rpc_types::TransactionReceipt>> {
		debug!(
			%hash,
			"aptos module JSON-RPC request to `eth_getTransactionReceipt`"
		);

		let mut accessory_state = working_set.accessory_state();

		let tx_number = self.transaction_hashes.get(&hash, &mut accessory_state);

		let receipt = tx_number.map(|number| {
			let tx = self
				.transactions
				.get(number as usize, &mut accessory_state)
				.expect("Transaction with known hash must be set");
			let block = self
				.blocks
				.get(tx.block_number as usize, &mut accessory_state)
				.expect("Block number for known transaction must be set");

			self.receipts
				.get(tx_number.unwrap() as usize, &mut accessory_state)
				.expect("Receipt for known transaction must be set")

			//build_rpc_receipt(block, tx, tx_number.unwrap(), receipt)
		});

		todo!()
		//Ok(receipt)
	}

	/// Handler for: `eth_call`
	//https://github.com/paradigmxyz/reth/blob/f577e147807a783438a3f16aad968b4396274483/crates/rpc/rpc/src/eth/api/transactions.rs#L502
	//https://github.com/paradigmxyz/reth/blob/main/crates/rpc/rpc-types/src/eth/call.rs#L7
	#[rpc_method(name = "eth_call")]
	pub fn get_call(
		&self,
		request: reth_rpc_types::TransactionRequest,
		block_number: Option<String>,
		_state_overrides: Option<reth_rpc_types::state::StateOverride>,
		_block_overrides: Option<Box<reth_rpc_types::BlockOverrides>>,
		working_set: &mut WorkingSet<S>,
	) -> RpcResult<reth_primitives::Bytes> {
		debug!("aptos module JSON-RPC request to `eth_call`");

		let block_env = match block_number {
			Some(ref block_number) if block_number == "pending" => {
				self.block_env.get(working_set).unwrap_or_default().clone()
			},
			_ => {
				let block = self.get_sealed_block_by_number(block_number, working_set);
				BlockEnv::from(block)
			},
		};

		// let tx_env = prepare_call_env(&block_env, request.clone()).unwrap();
		//
		// let cfg = self.cfg.get(working_set).unwrap_or_default();
		// let cfg_env = get_cfg_env_with_handler(&block_env, cfg, Some(get_cfg_env_template()));
		//
		// let aptos_db: AptosDb<'_, S> = self.get_db(working_set);

		// let result = match executor::inspect(aptos_db, &block_env, tx_env, cfg_env) {
		//     Ok(result) => result.result,
		//     Err(err) => return Err(AptosApiError::from(err).into()),
		// };

		// Ok(result?)
		todo!()
	}

	/// Handler for: `eth_blockNumber`
	#[rpc_method(name = "eth_blockNumber")]
	pub fn block_number(&self, working_set: &mut WorkingSet<S>) -> RpcResult<U256> {
		debug!("aptos module JSON-RPC request to `eth_blockNumber`");

		Ok(U256::from(self.blocks.len(&mut working_set.accessory_state()).saturating_sub(1)))
	}

	fn get_sealed_block_by_number(
		&self,
		_block_number: Option<String>,
		_working_set: &mut WorkingSet<S>,
	) -> BlockEnv {
		// safe, finalized, and pending are not supported
		BlockEnv::default()
	}
}

fn get_cfg_env_template() -> revm::primitives::CfgEnv {
	let mut cfg_env = revm::primitives::CfgEnv::default();
	// Reth sets this to true and uses only timeout, but other clients use this as a part of DOS attacks protection, with 100mln gas limit
	// https://github.com/paradigmxyz/reth/blob/62f39a5a151c5f4ddc9bf0851725923989df0412/crates/rpc/rpc/src/eth/revm_utils.rs#L215
	cfg_env.disable_block_gas_limit = false;
	cfg_env.disable_eip3607 = true;
	cfg_env.disable_base_fee = true;
	cfg_env.chain_id = 0;
	cfg_env.perf_analyse_created_bytecodes = revm::primitives::AnalysisKind::Analyse;
	cfg_env.limit_contract_code_size = None;
	cfg_env
}

// modified from: https://github.com/paradigmxyz/reth/blob/cc576bc8690a3e16e6e5bf1cbbbfdd029e85e3d4/crates/rpc/rpc/src/eth/api/transactions.rs#L849
pub(crate) fn build_rpc_receipt(
	block: SealedBlock,
	tx: TransactionSignedAndRecovered,
	tx_number: u64,
	receipt: Receipt,
) -> reth_rpc_types::TransactionReceipt {
	todo!()
}

fn map_out_of_gas_err<S: sov_modules_api::Spec>(
	block_env: BlockEnv,
	mut tx_env: revm::primitives::TxEnv,
	cfg_env_with_handler: revm::primitives::CfgEnvWithHandlerCfg,
	db: AptosDb<'_, S>,
) -> EthApiError {
	todo!()
}

fn convert_u256_to_u64(u256: U256) -> Result<u64, TryFromSliceError> {
	let bytes: [u8; 32] = u256.to_be_bytes();
	let bytes: [u8; 8] = bytes[24..].try_into()?;
	Ok(u64::from_be_bytes(bytes))
}
