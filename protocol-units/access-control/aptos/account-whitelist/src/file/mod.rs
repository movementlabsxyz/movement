use crate::WhitelistedAccountAddress;
use aptos_types::account_address::AccountAddress;
pub use whitelist::file::Whitelist as GenericWhitelist;
use whitelist::file::{Error, TryFromFileLine};
pub use whitelist::WhitelistOperations;

impl TryFromFileLine for WhitelistedAccountAddress {
	fn try_from_file_line(line: &str) -> Result<Self, Error>
	where
		Self: Sized,
	{
		Ok(Self(AccountAddress::from_hex(line).map_err(|e| Error::Internal(e.to_string()))?))
	}
}

pub type Whitelist = GenericWhitelist<WhitelistedAccountAddress>;
