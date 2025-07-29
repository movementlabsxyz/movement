pub mod ed25519;
pub mod secp256k1;

/// A marker trait for cryptography implementations that do not have a spec.
///
/// Note:
/// In theory, we could have an exploded signer spec where one type is specified for signing and the other for getting the public key. But, this adds little utility at this point.
pub trait LocalCryptographyNoSpec {}

/// Local cryptography specs have a curve associated with them.
pub trait LocalCryptographySpec {
	type Curve;
}
