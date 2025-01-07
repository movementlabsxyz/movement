use crate::cryptography::AwsKmsCryptographySpec;
use anyhow::Context;
use aws_sdk_kms::primitives::Blob;
use aws_sdk_kms::Client;
use movement_signer::cryptography::TryFromBytes;
use movement_signer::{cryptography::Curve, SignerError, Signing};
pub mod key;

/// An AWS KMS HSM.
pub struct AwsKms<C: Curve + AwsKmsCryptographySpec> {
	client: Client,
	key_id: String,
	public_key: <C as Curve>::PublicKey,
	_cryptography_marker: std::marker::PhantomData<C>,
}

impl<C> AwsKms<C>
where
	C: Curve + AwsKmsCryptographySpec,
{
	/// Creates a new AWS KMS HSM
	pub fn new(client: Client, key_id: String, public_key: C::PublicKey) -> Self {
		Self { client, key_id, public_key, _cryptography_marker: std::marker::PhantomData }
	}

	/// Sets the key id
	pub fn set_key_id(&mut self, key_id: String) {
		self.key_id = key_id;
	}

	/// Tries to create a new AWS KMS HSM from the environment
	pub async fn try_from_env() -> Result<Self, anyhow::Error> {
		let key_id = std::env::var("AWS_KMS_KEY_ID").context("AWS_KMS_KEY_ID not set")?;
		let public_key = std::env::var("AWS_KMS_PUBLIC_KEY").unwrap_or_default();

		let config = aws_config::load_from_env().await;
		let client = aws_sdk_kms::Client::new(&config);

		Ok(Self::new(client, key_id, C::PublicKey::try_from_bytes(public_key.as_bytes())?))
	}

	/// Creates in AWS KMS matching the provided key id.
	pub async fn create_key(self) -> Result<Self, anyhow::Error> {
		let res = self
			.client
			.create_key()
			.key_spec(C::key_spec())
			.key_usage(C::key_usage_type())
			.send()
			.await?;

		let key_id = res.key_metadata().context("No key metadata available")?.key_id().to_string();

		Ok(Self::new(self.client, key_id, self.public_key))
	}

	/// Fills the public key from the key id
	pub async fn fill_with_public_key(mut self) -> Result<Self, anyhow::Error> {
		let res = self.client.get_public_key().key_id(&self.key_id).send().await?;
		let public_key = C::PublicKey::try_from_bytes(
			res.public_key().context("No public key available")?.as_ref(),
		)?;
		self.public_key = public_key;
		Ok(self)
	}

	/// Gets a reference to the public key
	pub fn public_key(&self) -> &C::PublicKey {
		&self.public_key
	}
}

impl<C> Signing<C> for AwsKms<C>
where
	C: Curve + AwsKmsCryptographySpec + Sync,
{
	async fn sign(&self, message: &[u8]) -> Result<C::Signature, SignerError> {
		let blob = Blob::new(message);
		let request = self
			.client
			.sign()
			.key_id(&self.key_id)
			.signing_algorithm(C::signing_algorithm_spec())
			.message(blob);

		let res = request
			.send()
			.await
			.map_err(|e| SignerError::Internal(format!("Failed to sign: {}", e.to_string())))?;

		let signature = <C as Curve>::Signature::try_from_bytes(
			res.signature()
				.context("No signature available")
				.map_err(|e| {
					SignerError::Internal(format!("Failed to convert signature: {}", e.to_string()))
				})?
				.as_ref(),
		)
		.map_err(|e| {
			SignerError::Internal(format!("Failed to convert signature: {}", e.to_string()))
		})?;

		Ok(signature)
	}

	async fn public_key(&self) -> Result<C::PublicKey, SignerError> {
		let res = self.client.get_public_key().key_id(&self.key_id).send().await.map_err(|e| {
			SignerError::Internal(format!("failed to get public key: {}", e.to_string()))
		})?;
		let public_key = C::PublicKey::try_from_bytes(
			res.public_key()
				.context("No public key available")
				.map_err(|e| {
					SignerError::Internal(format!(
						"failed to convert public key: {}",
						e.to_string()
					))
				})?
				.as_ref(),
		)
		.map_err(|e| {
			SignerError::Internal(format!("Failed to convert public key: {}", e.to_string()))
		})?;
		Ok(public_key)
	}
}
