use crate::cryptography::Curve;

/// The secp256k1 elliptic curve.
#[derive(Debug, Clone)]
pub struct Secp256k1;

impl Curve for Secp256k1 {}
