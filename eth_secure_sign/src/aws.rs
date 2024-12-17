use crate::Bytes;
use crate::Hsm;
use crate::Signature;
use anyhow::Result;
use aws_sdk_kms::model::SigningAlgorithmSpec;
use aws_sdk_kms::output::{SignOutput, VerifyOutput};
use aws_sdk_kms::types::Blob;
use aws_sdk_kms::{Client, Config, Credentials, Region};

pub struct AwsKms {
	client: Client,
	key: String,
}

impl AwsKms {
	pub fn new(key_id: &str, access_key: &str, secret_key: &str) -> Self {
		let region = Region::new("us-east-1");
		let credentials = Credentials::new(
			access_key,
			secret_key,
			None, // No expiration time
			None, // No session token
			"CustomProvider",
		);
		let config = Config::builder().region(region).credentials_provider(credentials).build();

		let client = Client::from_conf(config);

		AwsKms { client, key: key_id.to_string() }
	}
}

#[async_trait::async_trait]
impl Hsm for AwsKms {
	async fn sign(&self, message: Bytes) -> Result<Signature> {
		let blob = Blob::new(message.0);
		let request = self
			.client
			.sign()
			.key_id(&self.key)
			.signing_algorithm(SigningAlgorithmSpec::EcdsaSha256)
			.message(blob);

		let response: SignOutput = request.send().await?;

		Ok(Signature(Bytes(response.signature().unwrap().as_ref().to_vec())))
	}

	async fn verify(&self, message: Bytes, signature: Signature) -> Result<bool> {
		let message_blob = Blob::new(message.0);
		let signature_blob = Blob::new(signature.0 .0);
		let request = self
			.client
			.verify()
			.key_id(&self.key)
			.signing_algorithm(SigningAlgorithmSpec::EcdsaSha256)
			.message(message_blob)
			.signature(signature_blob);

		let response: VerifyOutput = request.send().await?;

		Ok(response.signature_valid())
	}
}
