pub mod config;
pub mod file;
use aptos_types::account_address::AccountAddress;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct WhitelistedAccountAddress(pub AccountAddress);

impl WhitelistedAccountAddress {
	pub fn new(address: AccountAddress) -> Self {
		Self(address)
	}

	pub fn into_inner(self) -> AccountAddress {
		self.0
	}
}
