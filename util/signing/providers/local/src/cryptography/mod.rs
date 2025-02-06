pub mod ed25519;
pub mod secp256k1;

pub trait LocalCryptographySpec {
	type Curve;
}
