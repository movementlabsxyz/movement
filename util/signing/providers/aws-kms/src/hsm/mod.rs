use crate::cryptography::AwsKmsCryptographySpec;
use anyhow::Context;
use aws_sdk_kms::primitives::Blob;
use aws_sdk_kms::Client;
use movement_signer::cryptography::TryFromBytes;
use movement_signer::{cryptography::Curve, SignerError, Signing};
use secp256k1::ecdsa::Signature as Secp256k1Signature;
use secp256k1::Error as Secp256k1Error;
pub mod key;

/// An AWS KMS HSM.
pub struct AwsKms<C: Curve + AwsKmsCryptographySpec> {
	client: Client,
	key_id: String,
	_cryptography_marker: std::marker::PhantomData<C>,
}

impl<C> AwsKms<C>
where
	C: Curve + AwsKmsCryptographySpec,
{
	/// Creates a new AWS KMS HSM
	pub fn new(client: Client, key_id: String) -> Self {
		Self { client, key_id, _cryptography_marker: std::marker::PhantomData }
	}

	/// Tries to create a new AWS KMS HSM from the environment
	pub async fn try_from_env_with_key(key_id: String) -> Result<Self, anyhow::Error> {
		let config = aws_config::load_from_env().await;
		let client = aws_sdk_kms::Client::new(&config);
		Ok(AwsKms::new(client, key_id))
	}

	/// Sets the key id
	pub fn set_key_id(&mut self, key_id: String) {
		self.key_id = key_id;
	}

	/// Tries to create a new AWS KMS HSM from the environment
	pub async fn try_from_env() -> Result<Self, anyhow::Error> {
		let key_id = std::env::var("AWS_KMS_KEY_ID").context("AWS_KMS_KEY_ID not set")?;

		let config = aws_config::load_from_env().await;
		let client = aws_sdk_kms::Client::new(&config);

		Ok(Self::new(client, key_id))
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

		Ok(Self::new(self.client, key_id))
	}
}

#[async_trait::async_trait]
impl<C> Signing<C> for AwsKms<C>
where
	C: Curve + AwsKmsCryptographySpec + Sync,
{
	async fn sign(&self, message: &[u8]) -> Result<C::Signature, SignerError> {
		println!("Preparing to sign message. Message bytes: {:?}", message);

		let blob = Blob::new(message);
		// Todo: update to use Parameter Store to fetch Key Id
		let key_id = std::env::var("AWS_KMS_KEY_ID")
			.map_err(|_| SignerError::Internal("AWS_KMS_KEY_ID not set".to_string()))?;
		println!("Using Key ID: {}", key_id);
		let request = self
			.client
			.sign()
			.key_id(key_id)
			.signing_algorithm(C::signing_algorithm_spec())
			.message(blob);

		let res = request
			.send()
			.await
			.map_err(|e| SignerError::Internal(format!("Failed to sign: {}", e.to_string())))?;

		println!("Response signature (DER format): {:?}", res.signature());

		// Convert DER signature to raw format using secp256k1
		let der_signature = res
			.signature()
			.context("No signature available")
			.map_err(|e| SignerError::Internal(e.to_string()))?;

		let secp_signature =
			Secp256k1Signature::from_der(der_signature.as_ref()).map_err(|e: Secp256k1Error| {
				SignerError::Internal(format!("Failed to parse DER signature: {}", e))
			})?;

		let raw_signature = secp_signature.serialize_compact();
		println!("Raw signature: {:?}", raw_signature);

		// Convert the raw signature into the appropriate curve type
		let signature = <C as Curve>::Signature::try_from_bytes(&raw_signature).map_err(|e| {
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

// Utility function for DER-to-raw signature conversion
pub fn der_to_raw_signature(der: &[u8]) -> Result<[u8; 64], String> {
	if der.len() < 8 || der[0] != 0x30 {
		return Err("Invalid DER signature".to_string());
	}

	let r_len = der[3] as usize;
	let r_start = 4;
	let r_end = r_start + r_len;

	let s_len = der[r_end + 1] as usize;
	let s_start = r_end + 2;
	let s_end = s_start + s_len;

	if r_end > der.len() || s_end > der.len() {
		return Err("Invalid DER signature length".to_string());
	}

	// Extract `r` and `s`
	let r = &der[r_start..r_end];
	let s = &der[s_start..s_end];

	// Ensure `r` and `s` are 32 bytes by trimming leading zeros
	let mut raw_r = [0u8; 32];
	let mut raw_s = [0u8; 32];

	if r.len() > 32 {
		return Err("Invalid r length".to_string());
	}
	if s.len() > 32 {
		return Err("Invalid s length".to_string());
	}

	raw_r[32 - r.len()..].copy_from_slice(r);
	raw_s[32 - s.len()..].copy_from_slice(s);

	// Combine `r` and `s` into a 64-byte raw signature
	let mut raw_signature = [0u8; 64];
	raw_signature[..32].copy_from_slice(&raw_r);
	raw_signature[32..].copy_from_slice(&raw_s);

	Ok(raw_signature)
}
