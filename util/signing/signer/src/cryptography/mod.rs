pub mod ed25519;
pub mod secp256k1;

/// A curve.
/// Currently this has not methods, but it is used to bound the `Signer` trait.
pub trait Curve {}
