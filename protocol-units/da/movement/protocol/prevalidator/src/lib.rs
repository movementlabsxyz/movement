pub mod aptos;

use thiserror::Error;

/// Domain error for the transaction pipe task
#[derive(Debug, Error)]
pub enum Error {
	#[error("prevalidator internal error: {0}")]
	Internal(String),
	#[error("prevalidator validation error: {0}")]
	Validation(String),
}

#[derive(Debug)]
/// A prevalidated outcome. Indicates that input of A (from the trait [PrevalidatorOperations]) is prevalidated as an instance of B, or else invalid instance.
pub struct Prevalidated<B>(pub B);

impl<B> Prevalidated<B> {
	pub fn new(blob: B) -> Self {
		Self(blob)
	}

	pub fn inner(&self) -> &B {
		&self.0
	}

	pub fn into_inner(self) -> B {
		self.0
	}
}
