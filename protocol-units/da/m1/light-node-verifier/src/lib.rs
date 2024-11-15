pub mod celestia;
pub mod permissioned_signers;
pub mod signed;

pub use m1_da_light_node_grpc::*;
use thiserror::Error;

/// Domain error for the transaction pipe task
#[derive(Debug, Error)]
pub enum Error {
	#[error("verifier internal error: {0}")]
	Internal(String),
	#[error("verifier validation error: {0}")]
	Validation(String),
}

/// thiserror for validation and internal errors
#[derive(thiserror::Error, Debug)]

/// A verified outcome. Indicates that input of A (from the trait [VerifierOperations]) is verified as valid instance of B, or else invalid instance.
pub struct Verified<B>(B);

impl<B> Verified<B> {
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

#[tonic::async_trait]
pub trait VerifierOperations<A, B>
where
	A: Send + Sync + 'static,
	B: Send + Sync + 'static,
{
	async fn verify(&self, blob: A, height: u64) -> Result<Verified<B>, Error>;
}
