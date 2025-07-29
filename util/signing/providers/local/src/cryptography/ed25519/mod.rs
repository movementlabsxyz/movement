//! ed25519 is unimplemented for local cryptography spec
use crate::cryptography::LocalCryptographyNoSpec;
use movement_signer::cryptography::ed25519::Ed25519;

impl LocalCryptographyNoSpec for Ed25519 {}
