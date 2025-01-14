use crate::cryptography::AwsKmsCryptographySpec;
use anyhow::Context;
use aws_sdk_kms::error::ProvideErrorMetadata;
use aws_sdk_kms::operation::RequestId;
use aws_sdk_kms::primitives::Blob;
use aws_sdk_kms::Client;
use movement_signer::cryptography::TryFromBytes;
use movement_signer::{cryptography::Curve, SignerError, Signing};
use secp256k1::ecdsa::Signature as Secp256k1Signature;
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

	/// Sets the key id
	pub fn set_key_id(&mut self, key_id: String) {
		self.key_id = key_id;
	}

	/// Tries to create a new AWS KMS HSM from the environment
	pub async fn try_from_env() -> Result<Self, anyhow::Error> {
		let key_id = std::env::var("AWS_KMS_KEY_ID").unwrap_or("0x1".to_string());
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

                let key_metadata = self
                        .client
                        .describe_key()
                        .key_id(&alias_name)
                        .send()
                        .await
                        .map_err(|e| {
                                println!("Error resolving key ID from alias: {:?}", e);
                                SignerError::Internal(format!(
                                        "Failed to resolve key ID: {:?}",
                                        e
                                ))
                        })?;

		let key_id = key_metadata
			.key_metadata()
			.and_then(|metadata| Some(metadata.key_id()))
			.map(|key_id| key_id.to_string()) 
			.ok_or_else(|| {
			    	SignerError::Internal("Failed to retrieve key ID".to_string())
			})?;

                Ok(key_id)
        }
}

impl<C> Signing<C> for AwsKms<C>
where
	C: Curve + AwsKmsCryptographySpec + Sync,
{
	async fn sign(&self, message: &[u8]) -> Result<C::Signature, SignerError> {
		println!("Preparing to sign message. Message bytes: {:?}", message);

		// Ensure the alias has the correct prefix
		let key_alias = format!("alias/{}", self.key_id);
		println!("Using Key Alias: {}", key_alias);

		// Convert the message into a Blob
		let blob = Blob::new(message);

		// Use the `sign` API with the alias directly
		let res = self
			.client
			.sign()
			.key_id(&key_alias)
			.signing_algorithm(C::signing_algorithm_spec())
			.message(blob)
			.send()
			.await
			.map_err(|e| {
				// Log detailed error information
				if let Some(service_error) = e.as_service_error() {
					let code = service_error.code().unwrap_or("Unknown").to_string();
					let message =
						service_error.message().unwrap_or("No message provided").to_string();
					let request_id = service_error
						.request_id()
						.map(|id| id.to_string())
						.unwrap_or_else(|| "No Request ID".to_string());

					println!(
						"AWS Service Error: Code: {}, Message: {}, Request ID: {}",
						code, message, request_id
					);
				} else {
					// Non-service error handling
					println!("Non-service error occurred: {:?}", e);
				}

				// Return a formatted error
				SignerError::Internal(format!("Failed to sign: {:?}", e))
			})?;

		println!("Response signature (DER format): {:?}", res.signature());

		// Extract the DER-encoded signature
		let der_signature = res
			.signature()
			.context("No signature available")
			.map_err(|e| SignerError::Internal(e.to_string()))?;

		// Convert DER signature to raw format using secp256k1
		let secp_signature = Secp256k1Signature::from_der(der_signature.as_ref())
			.map_err(|e| SignerError::Internal(format!("Failed to parse DER signature: {}", e)))?;

		let raw_signature = secp_signature.serialize_compact();
		println!("Raw signature: {:?}", raw_signature);

		// Convert the raw signature into the appropriate curve type
		let signature = <C as Curve>::Signature::try_from_bytes(&raw_signature).map_err(|e| {
			SignerError::Internal(format!("Failed to convert signature: {}", e.to_string()))
		})?;

		Ok(signature)
	}

	async fn public_key(&self) -> Result<C::PublicKey, SignerError> {
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
				SignerError::Internal(format!("Failed to get public key: {:?}", e))
			})?;
	
		println!("get_public_key response: {:?}", res);
	
		let public_key_bytes = res
			.public_key()
			.context("No public key available")
			.map_err(|e| {
				println!("Error extracting public key: {:?}", e);
				SignerError::Internal(e.to_string())
			})?;
	
		let public_key = C::PublicKey::try_from_bytes(public_key_bytes.as_ref()).map_err(|e| {
			println!("Error converting public key bytes: {:?}", e);
			SignerError::Internal(e.to_string())
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
