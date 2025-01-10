use crate::cryptography::LocalCryptographySpec;
use movement_signer::cryptography::secp256k1::Secp256k1;

impl LocalCryptographySpec for Secp256k1 {
	type Curve = k256::Secp256k1;
}
