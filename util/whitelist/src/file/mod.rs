use crate::{Error as WhitelistOperationsError, WhitelistOperations};
use std::collections::HashSet;
use thiserror::Error;

/// Domain error for the file line parser
#[derive(Debug, Error)]
pub enum Error {
	#[error("prevalidator internal error: {0}")]
	Internal(String),
}

/// Tries to read the type from a line matching the format.
pub trait TryFromFileLine {
	fn try_from_file_line(line: &str) -> Result<Self, Error>
	where
		Self: Sized;
}

/// A whitelist that reads a hashset from a file.
pub struct Whitelist<T: TryFromFileLine> {
	whitelist: HashSet<T>,
}

impl<T> Whitelist<T>
where
	T: TryFromFileLine + std::hash::Hash + Eq,
{
	/// Creates a new whitelist from a file.
	pub fn try_new(file_path: &str) -> Result<Self, Error> {
		let whitelist = std::fs::read_to_string(file_path)
			.map_err(|e| Error::Internal(format!("Failed to read file: {}", e)))?
			.lines()
			.map(|line| T::try_from_file_line(line))
			.collect::<Result<HashSet<T>, Error>>()?;

		Ok(Self { whitelist })
	}
}

#[tonic::async_trait]
impl<T> WhitelistOperations<T> for Whitelist<T>
where
	T: TryFromFileLine + std::hash::Hash + Eq + Send + Sync + 'static,
{
	/// Checks if the item is whitelisted.
	async fn is_whitelisted(&self, item: &T) -> Result<bool, WhitelistOperationsError> {
		Ok(self.whitelist.contains(item))
	}

	/// Converts the whitelist to a hashset.
	fn try_into_set(self) -> Result<HashSet<T>, WhitelistOperationsError> {
		Ok(self.whitelist)
	}
}
