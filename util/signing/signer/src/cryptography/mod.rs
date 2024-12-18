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
	};
}

pub mod ed25519;
pub mod secp256k1;

/// A designator for an elliptic curve.
///
/// This trait has no methods, but it binds the types of the public key and
/// the signature used by the EC digital signing algorithm.
pub trait Curve {
	type PublicKey;
	type Signature;
}
