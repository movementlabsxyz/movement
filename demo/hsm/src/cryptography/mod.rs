pub mod aws_kms;
pub mod google_kms;
pub mod hashicorp_vault;
pub mod verifier;

/// The Secp256k1 curve.
#[derive(Debug, Clone, Copy)]
pub struct Secp256k1;

/// The Ed25519 curve.
#[derive(Debug, Clone, Copy)]
pub struct Ed25519;
