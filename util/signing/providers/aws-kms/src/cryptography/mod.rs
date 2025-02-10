pub mod secp256k1;
use aws_sdk_kms::types::{KeySpec, KeyUsageType, SigningAlgorithmSpec};

/// Defines the needed methods for providing a definition of cryptography used with AWS KMS
pub trait AwsKmsCryptographySpec {
	/// Returns the [KeySpec] for the desired cryptography
	fn key_spec() -> KeySpec;

	/// Returns the [KeyUsageType] for the desired cryptography
	fn key_usage_type() -> KeyUsageType;

	/// Returns the [SigningAlgorithmSpec] for the desired cryptography
	fn signing_algorithm_spec() -> SigningAlgorithmSpec;
}
