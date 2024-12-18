use crate::cryptography::Curve;

/// The Ed25519 curve.
#[derive(Debug, Clone)]
pub struct Ed25519;

impl Curve for Ed25519 {}
