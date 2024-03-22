use std::collections::HashMap;

use anyhow::Result;
use reth_primitives::{Bloom, Bytes, EMPTY_OMMER_ROOT_HASH, KECCAK_EMPTY};
use revm::primitives::{Address, SpecId, B256, U256};
use sov_modules_api::{DaSpec, WorkingSet};

use crate::experimental::AptosVM;

/// Evm account.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, Eq, PartialEq)]
pub struct AccountData {
	/// Account address.
	pub address: Address,
	/// Account balance.
	pub balance: U256,
	/// Code hash.
	pub code_hash: B256,
	/// Smart contract code.
	pub code: Bytes,
	/// Account nonce.
	pub nonce: u64,
}

impl AccountData {
	/// Empty code hash.
	pub fn empty_code() -> B256 {
		KECCAK_EMPTY
	}

	/// Account balance.
	pub fn balance(balance: u64) -> U256 {
		U256::from(balance)
	}
}

/// Genesis configuration.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, Eq, PartialEq)]
pub struct AptosConfig {
	/// Genesis accounts.
	pub data: Vec<AccountData>,
	/// Chain id.
	pub chain_id: u64,
	/// Limits size of contract code size.
	pub limit_contract_code_size: Option<usize>,
	/// List of EVM hard forks by block number
	pub spec: HashMap<u64, SpecId>,
	/// Coinbase where all the fees go
	pub coinbase: Address,
	/// Starting base fee.
	pub starting_base_fee: u64,
	/// Gas limit for single block
	pub block_gas_limit: u64,
	/// Genesis timestamp.
	pub genesis_timestamp: u64,
	/// Delta to add to parent block timestamp,
	pub block_timestamp_delta: u64,
	/// Base fee params.
	pub base_fee_params: reth_primitives::BaseFeeParams,
}

impl Default for AptosConfig {
	fn default() -> Self {
		Self {
			data: vec![],
			chain_id: 1,
			limit_contract_code_size: None,
			spec: vec![(0, SpecId::SHANGHAI)].into_iter().collect(),
			coinbase: Address::ZERO,
			starting_base_fee: reth_primitives::constants::MIN_PROTOCOL_BASE_FEE,
			block_gas_limit: reth_primitives::constants::ETHEREUM_BLOCK_GAS_LIMIT,
			block_timestamp_delta: reth_primitives::constants::SLOT_DURATION.as_secs(),
			genesis_timestamp: 0,
			base_fee_params: reth_primitives::BaseFeeParams::ethereum(),
		}
	}
}

impl<S: sov_modules_api::Spec, Da: DaSpec> AptosVM<S, Da> {
	pub(crate) fn init_module(
		&self,
		config: &<Self as sov_modules_api::Module>::Config,
		working_set: &mut WorkingSet<S>,
	) -> Result<()> {
		Ok(())
	}
}
