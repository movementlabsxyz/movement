use crate::{Bytes, Hsm, PublicKey, Signature};
use anyhow::Context;
use aws_sdk_kms::primitives::Blob;
use aws_sdk_kms::types::{KeySpec, KeyUsageType, SigningAlgorithmSpec};
use aws_sdk_kms::Client;
use k256::ecdsa::{self, VerifyingKey};
use k256::pkcs8::DecodePublicKey;
use ring_compat::signature::Verifier;

/// A AWS KMS HSM.
pub struct AwsKms {
	client: Client,
	key_id: String,
	public_key: PublicKey,
}

impl AwsKms {
	/// Creates a new AWS KMS HSM
	pub fn new(client: Client, key_id: String, public_key: PublicKey) -> Self {
		Self { client, key_id, public_key }
	}

	/// Tries to create a new AWS KMS HSM from the environment
	pub async fn try_from_env() -> Result<Self, anyhow::Error> {
		let key_id = std::env::var("AWS_KMS_KEY_ID").context("AWS_KMS_KEY_ID not set")?;
		let public_key = std::env::var("AWS_KMS_PUBLIC_KEY").unwrap_or_default();

		let config = aws_config::load_from_env().await;
		let client = aws_sdk_kms::Client::new(&config);

		Ok(Self::new(client, key_id, PublicKey(Bytes(public_key.as_bytes().to_vec()))))
	}

	/// Creates in AWS KMS matching the provided key id.
	pub async fn create_key(self) -> Result<Self, anyhow::Error> {
		let res = self
			.client
			.create_key()
			.key_spec(KeySpec::EccSecgP256K1)
			.key_usage(KeyUsageType::SignVerify)
			.send()
			.await?;

		let key_id = res.key_metadata().context("No key metadata available")?.key_id().to_string();

		Ok(Self::new(self.client, key_id, self.public_key))
	}

	/// Fills the public key from the key id
	pub async fn fill_with_public_key(mut self) -> Result<Self, anyhow::Error> {
		let res = self.client.get_public_key().key_id(&self.key_id).send().await?;
		let public_key = PublicKey(Bytes(
			res.public_key().context("No public key available")?.as_ref().to_vec(),
		));
		self.public_key = public_key;
		Ok(self)
	}
}

#[async_trait::async_trait]
impl Hsm for AwsKms {
	async fn sign(&self, message: Bytes) -> Result<(Bytes, PublicKey, Signature), anyhow::Error> {
		let blob = Blob::new(message.clone().0);
		let request = self
			.client
			.sign()
			.key_id(&self.key_id)
			.signing_algorithm(SigningAlgorithmSpec::EcdsaSha256)
			.message(blob);

		let res = request.send().await?;
		println!("res: {:?}", res);
		let signature =
			Signature(Bytes(res.signature().context("No signature available")?.as_ref().to_vec()));

		Ok((message, self.public_key.clone(), signature))
	}

	async fn verify(
		&self,
		message: Bytes,
		public_key: PublicKey,
		signature: Signature,
	) -> Result<bool, anyhow::Error> {
		let verifying_key = VerifyingKey::from_public_key_der(&public_key.0 .0)
			.context("Failed to create verifying key")?;

		let signature =
			ecdsa::Signature::from_der(&signature.0 .0).context("Failed to create signature")?;

		match verifying_key.verify(message.0.as_slice(), &signature) {
			Ok(_) => Ok(true),
			Err(e) => {
				println!("Error verifying signature: {:?}", e);
				Ok(false)
			}
		}
	}
}
