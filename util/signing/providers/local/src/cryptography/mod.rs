pub mod ed25519;
pub mod secp256k1;

/// A marker trait for cryptography implementations that do not have a spec.
pub trait LocalCryptographyNoSpec {}

/// Local cryptography specs have a curve associated with them.
pub trait LocalCryptographySpec {
	type Curve;
}
