use crate::{cryptography, Signing};
use std::error;

/// Errors thrown by [KeyManager].
#[derive(Debug, thiserror::Error)]
pub enum KeyManagerError {
	#[error("building signer failed")]
	BuildingSigner(#[source] Box<dyn error::Error + Send + Sync>),
}

pub trait KeyManager<C: cryptography::Curve, S: Signing<C>> {
	/// A key manager is bound by a specific identifier type.
	/// the Curve (C) and the Signing (S) types are generic because it is presumed the key manager can handle multiple (generic) curves and signing services, but potentially only one key identifier type.
	type Id;

	/// Get async signer for a key.
	fn build_signer(&self, key_id: Self::Id) -> Result<S, KeyManagerError>;
}
