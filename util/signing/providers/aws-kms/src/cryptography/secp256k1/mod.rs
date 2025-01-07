use crate::cryptography::AwsKmsCryptographySpec;
use aws_sdk_kms::types::{KeySpec, KeyUsageType, SigningAlgorithmSpec};
use movement_signer::cryptography::secp256k1::Secp256k1;

impl AwsKmsCryptographySpec for Secp256k1 {
	fn key_spec() -> KeySpec {
		KeySpec::EccSecgP256K1
	}

	fn key_usage_type() -> KeyUsageType {
		KeyUsageType::SignVerify
	}

	fn signing_algorithm_spec() -> SigningAlgorithmSpec {
		SigningAlgorithmSpec::EcdsaSha256
	}
}
