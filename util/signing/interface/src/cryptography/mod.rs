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

		impl crate::cryptography::TryFromBytes for $Name {
			fn try_from_bytes(bytes: &[u8]) -> Result<Self, anyhow::Error> {
				if bytes.len() != Self::BYTES_LEN {
					Err(anyhow::anyhow!("invalid length"))?;
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

pub trait TryFromBytes: Sized {
	fn try_from_bytes(bytes: &[u8]) -> Result<Self, anyhow::Error>;
}

/// A designator for an elliptic curve.
///
/// This trait has no methods, but it binds the types of the public key and
/// the signature used by the EC digital signing algorithm.
pub trait Curve {
	type PublicKey: TryFromBytes;
	type Signature;
}

/// Errors that occur when parsing signature or key material from byte sequences.
#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct CryptoMaterialError(Box<dyn Error + Send + Sync>);
