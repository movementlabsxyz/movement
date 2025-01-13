macro_rules! fixed_size {
	(pub struct $Name:ident([u8; $len:expr])) => {
		#[derive(Copy, Clone, Debug, PartialEq, Eq)]
		pub struct $Name([u8; Self::BYTES_LEN]);

		impl $Name {
			pub const BYTES_LEN: usize = $len;

			pub fn as_bytes(&self) -> &[u8] {
				&self.0
			}
		}

		impl TryFrom<&[u8]> for $Name {
			type Error = crate::cryptography::CryptoMaterialError;

			fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
				use crate::cryptography::CryptoMaterialError;

				if bytes.len() != Self::BYTES_LEN {
					Err(CryptoMaterialError("invalid length".into()))?;
				}

				let mut inner = [0u8; Self::BYTES_LEN];
				inner.copy_from_slice(bytes);

				Ok(Self(inner))
			}
		}
	};
}

pub mod ed25519;
pub mod secp256k1;

use std::error::Error;

/// A designator for an elliptic curve.
///
/// This trait has no methods, but it binds the types of the public key and
/// the signature used by the EC digital signing algorithm.
pub trait Curve {
	type PublicKey;
	type Signature;
}

/// Errors that occur when parsing signature or key material from byte sequences.
#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct CryptoMaterialError(Box<dyn Error + Send + Sync>);
