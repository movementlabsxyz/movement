use crate::cryptography::AwsKmsCryptographySpec;
use anyhow::Context;
use aws_sdk_kms::primitives::Blob;
use aws_sdk_kms::Client;
use movement_signer::cryptography::TryFromBytes;
use movement_signer::{cryptography::Curve, SignerError, Signing};
use secp256k1::ecdsa::Signature as Secp256k1Signature;
pub mod key;
use simple_asn1::{from_der, ASN1Block};

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

	/// Creates a randomly named key in AWS KMS
	pub async fn create_random_key() -> Result<Self, anyhow::Error> {
		let config = aws_config::load_from_env().await;
		let client = aws_sdk_kms::Client::new(&config);

		let res = client
			.create_key()
			.key_spec(C::key_spec())
			.key_usage(C::key_usage_type())
			.send()
			.await?;

		let key_id = res.key_metadata().context("No key metadata available")?.key_id().to_string();

		Ok(Self::new(client, key_id))
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
		let key_id = std::env::var("AWS_KMS_KEY_ID").unwrap_or_default();

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

	pub async fn resolve_key_id(&self) -> Result<String, SignerError> {
		let alias_name = format!("alias/{}", self.key_id); // Ensure the alias format is correct

		let key_metadata =
			self.client.describe_key().key_id(&alias_name).send().await.map_err(|e| {
				println!("Error resolving key ID from alias: {:?}", e);
				SignerError::Internal(format!("Failed to resolve key ID: {:?}", e))
			})?;

		let key_id = key_metadata
			.key_metadata()
			.and_then(|metadata| Some(metadata.key_id()))
			.map(|key_id| key_id.to_string())
			.ok_or_else(|| SignerError::Internal("Failed to retrieve key ID".to_string()))?;

		Ok(key_id)
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
		let request = self
			.client
			.sign()
			.key_id(self.key_id.clone())
			.signing_algorithm(C::signing_algorithm_spec())
			.message(blob);

		let res = request
			.send()
			.await
			.map_err(|e| SignerError::Internal(format!("Failed to sign: {}", e.to_string())))?;

		// Convert DER signature to raw format using secp256k1
		let der_signature = res
			.signature()
			.context("No signature available")
			.map_err(|e| SignerError::Internal(e.to_string()))?;

		let secp_signature = Secp256k1Signature::from_der(der_signature.as_ref())
			.map_err(|e| SignerError::Internal(format!("Failed to parse DER signature: {}", e)))?;

		let raw_signature = secp_signature.serialize_compact();

		// Convert the raw signature into the appropriate curve type
		let signature = <C as Curve>::Signature::try_from_bytes(&raw_signature).map_err(|e| {
			SignerError::Internal(format!("Failed to convert signature: {}", e.to_string()))
		})?;

		Ok(signature)
	}

	async fn public_key(&self) -> Result<C::PublicKey, SignerError> {
		// Resolve the Key ID
		let key_id = self.resolve_key_id().await.map_err(|e| {
			println!("Error resolving key ID: {:?}", e);
			SignerError::Internal(format!("Failed to resolve key ID: {:?}", e))
		})?;

		let res = self
			.client
			.get_public_key()
			.key_id(&key_id) // Use the resolved Key ID
			.send()
			.await
			.map_err(|e| {
				println!("Error calling get_public_key: {:?}", e);
				SignerError::Internal(format!("Failed to retrieve public key: {:?}", e))
			})?;

		let public_key_der = res.public_key().context("No public key available").map_err(|e| {
			println!("Error extracting public key: {:?}", e);
			SignerError::Internal(e.to_string())
		})?;

		// Convert the DER-encoded public key to raw format
		let raw_public_key = extract_raw_public_key(public_key_der.as_ref()).map_err(|e| {
			println!("Error decoding DER-encoded public key: {:?}", e);
			SignerError::Internal(e.to_string())
		})?;

		let public_key = C::PublicKey::try_from_bytes(raw_public_key.as_ref()).map_err(|e| {
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

fn extract_raw_public_key(der: &[u8]) -> Result<Vec<u8>, anyhow::Error> {
	let asn1_blocks = from_der(der).context("Failed to parse DER-encoded public key")?;

	if let Some(ASN1Block::Sequence(_, blocks)) = asn1_blocks.get(0) {
		if let Some(ASN1Block::BitString(_, _, key_bytes)) = blocks.get(1) {
			// Ensure the key is in uncompressed format
			if key_bytes.len() == 65 && key_bytes[0] == 4 {
				return Ok(key_bytes[0..].to_vec()); // Return the X and Y coordinate only
			} else {
				return Err(anyhow::anyhow!(
					"Unexpected public key format or length: {:?}",
					key_bytes
				));
			}
		}
	}

	Err(anyhow::anyhow!("Failed to extract raw public key from DER"))
}

#[cfg(test)]
pub mod test {

	use super::*;
	use movement_signer::cryptography::secp256k1::Secp256k1;
	use movement_signer::{Signing, Verify};

	#[tokio::test]
	async fn test_signing_and_verifying_secp256k1() -> Result<(), anyhow::Error> {
		let key = AwsKms::<Secp256k1>::create_random_key().await?;
		let message = b"Hello, world!";
		let signature = key.sign(message).await?;
		let public_key = key.public_key().await?;

		assert!(Secp256k1::verify(message, &signature, &public_key)?);

		Ok(())
	}
}
