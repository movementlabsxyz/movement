use std::convert::Infallible;
use aptos_sdk::types::account_address::AccountAddress;
use aptos_db::AptosDB as AptosDatabase;

use reth_primitives::Bytes;
use revm::primitives::{AccountInfo as ReVmAccountInfo, Address, Bytecode, B256, U256};
use revm::Database;
use sov_modules_api::{StateMapAccessor, WorkingSet};
use sov_state::codec::BcsCodec;

use super::DbAccount;

// This needs to change to use Aptos Types and we could try implementing revm::Database for AptosDB
pub(crate) struct AptosDb<'a, S: sov_modules_api::Spec> {
    pub(crate) accounts: sov_modules_api::StateMap<Address, DbAccount, BcsCodec>,
    pub(crate) code: sov_modules_api::StateMap<B256, Bytes, BcsCodec>,
    pub(crate) working_set: &'a mut WorkingSet<S>,
}

impl<'a, S: sov_modules_api::Spec> AptosDb<'a, S> {
    pub(crate) fn new(
        accounts: sov_modules_api::StateMap<Address, DbAccount, BcsCodec>,
        code: sov_modules_api::StateMap<B256, Bytes, BcsCodec>,
        working_set: &'a mut WorkingSet<S>,
    ) -> Self {
        Self {
            accounts,
            code,
            working_set,
        }
    }
}

impl<'a, S: sov_modules_api::Spec> Database for AptosDb<'a, S> {
    type Error = Infallible;

    fn basic(&mut self, address: Address) -> Result<Option<ReVmAccountInfo>, Self::Error> {
        let db_account = self.accounts.get(&address, self.working_set);
        Ok(db_account.map(|acc| acc.info.into()))
    }

    fn code_by_hash(&mut self, code_hash: B256) -> Result<Bytecode, Self::Error> {
        // TODO move to new_raw_with_hash for better performance
        let bytecode = Bytecode::new_raw(
            self.code
                .get(&code_hash, self.working_set)
                .unwrap_or_default(),
        );

        Ok(bytecode)
    }

    fn storage(&mut self, address: Address, index: U256) -> Result<U256, Self::Error> {
        let storage_value: U256 = if let Some(acc) = self.accounts.get(&address, self.working_set) {
            acc.storage
                .get(&index, self.working_set)
                .unwrap_or_default()
        } else {
            U256::default()
        };

        Ok(storage_value)
    }

    fn block_hash(&mut self, _number: U256) -> Result<B256, Self::Error> {
        todo!("block_hash not yet implemented")
    }
}