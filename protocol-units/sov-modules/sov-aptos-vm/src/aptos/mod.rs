// Much of this code was copy-pasted from reth-aptos, and we'd rather keep it as
// similar as possible to upstream than clean it up.
#![allow(clippy::match_same_arms)]

use revm::primitives::U256;
use serde::{Deserialize, Serialize};
use sov_modules_api::StateMap;
use sov_state::Prefix;

//pub(crate) mod call;
mod context;
pub(crate) mod db;
pub(crate) mod db_init;
pub(crate) mod error;
pub(crate) mod executor;
pub(crate) mod primitive_types;

use aptos_api_types::{Address, HexEncodedBytes, MoveModuleBytecode, MoveResource, U64};
use aptos_sdk::types::account_address::AccountAddress;
pub use primitive_types::RlpEvmTransaction;
use sov_state::codec::BcsCodec;

const PLACEHOLDER_APTOS_BLOCK_LIMIT: u64 = 1000;
const PLACEHOLDER_APTOS_CHAIN_ID: u64 = 1;
const PLACEHOLDER_APTOS_BLOCK_TIMESTAMP_DELTA: u64 = 1;
const PLACEHOLDER_APTOS_BASE_FEE: u64 = 0;

// Stores information about an Aptos account
#[derive(Deserialize, Serialize, Debug, Clone)]
pub(crate) struct AccountInfo {
	pub(crate) public_key: HexEncodedBytes,
	pub(crate) resources: Vec<MoveModuleBytecode>,
	pub(crate) modules: Vec<MoveResource>,
	pub(crate) sequence_number: U64,
}

/// Stores information about an Aptos account and a corresponding account state.
#[derive(Deserialize, Serialize, Debug, Clone)]
pub(crate) struct DbAccount {
	pub(crate) info: AccountInfo,
	pub(crate) storage: StateMap<U256, U256, BcsCodec>,
}

impl DbAccount {
	fn new(parent_prefix: &Prefix, address: Address) -> Self {
		let prefix = Self::create_storage_prefix(parent_prefix, address);
		Self {
			info: AccountInfo {
				public_key: HexEncodedBytes::from(vec![0]),
				resources: Vec::new(),
				modules: Vec::new(),
				sequence_number: U64::default(),
			},
			storage: StateMap::with_codec(prefix, BcsCodec {}),
		}
	}

	pub(crate) fn new_with_info(
		parent_prefix: &Prefix,
		address: Address,
		info: AccountInfo,
	) -> Self {
		let prefix = Self::create_storage_prefix(parent_prefix, address);
		Self { info, storage: StateMap::with_codec(prefix, BcsCodec {}) }
	}

	// Not sure what this function does for the aptos-module
	fn create_storage_prefix(parent_prefix: &Prefix, address: Address) -> Prefix {
		todo!()
	}
}

/// aptos Chain configuration
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct AptosChainConfig {
	/// Unique chain id
	/// Chains can be registered at <https://aptos.dev/nodes/networks/>.
	pub chain_id: u64,

	/// Limits size of module code size
	/// By default it is 0x6000 (~25kb).
	pub limit_module_code_size: Option<usize>,

	/// Coinbase where all the fees go
	pub coinbase: Address,

	/// Gas limit for single block
	pub block_gas_limit: u64,

	/// Delta to add to parent block timestamp
	pub block_timestamp_delta: u64,

	/// Base fee
	pub base_fee: u64,
}

impl Default for AptosChainConfig {
	fn default() -> AptosChainConfig {
		AptosChainConfig {
			chain_id: 1,
			limit_module_code_size: None,
			coinbase: Address::from(AccountAddress::random()),
			block_gas_limit: PLACEHOLDER_APTOS_BLOCK_LIMIT,
			block_timestamp_delta: PLACEHOLDER_APTOS_BLOCK_TIMESTAMP_DELTA,
			base_fee: PLACEHOLDER_APTOS_BASE_FEE,
		}
	}
}
