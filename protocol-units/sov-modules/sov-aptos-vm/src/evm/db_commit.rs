use revm::primitives::{Account, Address, HashMap};
use revm::DatabaseCommit;

use super::db::EvmDb;
use super::DbAccount;

impl<'a, S: sov_modules_api::Spec> DatabaseCommit for EvmDb<'a, S> {
    fn commit(&mut self, changes: HashMap<Address, Account>) {
        for (address, account) in changes {
            // TODO figure out what to do when account is destroyed.
            // https://github.com/Sovereign-Labs/sovereign-sdk/issues/425
            if account.is_selfdestructed() {
                todo!("Account destruction not supported")
            }

            let accounts_prefix = self.accounts.prefix();

            let mut db_account = self
                .accounts
                .get(&address, self.working_set)
                .unwrap_or_else(|| DbAccount::new(accounts_prefix, address));

            let account_info = account.info;

            if let Some(ref code) = account_info.code {
                if !code.is_empty() {
                    // TODO: would be good to have a contains_key method on the StateMap that would be optimized, so we can check the hash before storing the code
                    self.code
                        .set(&account_info.code_hash, &code.bytecode, self.working_set);
                }
            }

            db_account.info = account_info.into();

            for (key, value) in account.storage.into_iter() {
                let value = value.present_value();
                db_account.storage.set(&key, &value, self.working_set);
            }

            self.accounts.set(&address, &db_account, self.working_set);
        }
    }
}