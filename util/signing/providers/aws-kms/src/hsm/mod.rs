use aws_sdk_kms::primitives::Blob;
use aws_sdk_kms::types::KeySpec;
use aws_sdk_kms::types::KeyUsageType;
use aws_sdk_kms::types::SigningAlgorithmSpec;
use aws_sdk_kms::Client;
use k256::ecdsa;
use movement_signer::cryptography::secp256k1::{self, Secp256k1};
use movement_signer::cryptography::TryFromBytes;
use movement_signer::SignerError;
use movement_signer::Signing;

/// A AWS KMS HSM.
#[derive(Debug, Clone)]
pub struct AwsKmsSigner {
	pub client: Client,
	key_id: String,
}

impl Signing<Secp256k1> for AwsKmsSigner {
	/// Signs some bytes.
	async fn sign(&self, message: &[u8]) -> Result<secp256k1::Signature, SignerError> {
		let res = self
			.client
			.sign()
			.key_id(&self.key_id)
			.message(Blob::new(message))
			.message_type(aws_sdk_kms::types::MessageType::Digest)
			.signing_algorithm(SigningAlgorithmSpec::EcdsaSha256)
			.send()
			.await
			.map_err(|e| SignerError::Internal(e.to_string()))?;

		// Decode signature.
		let sign_bytes = res.signature().ok_or(SignerError::KeyNotFound)?.as_ref();
		let sig =
			ecdsa::Signature::from_der(sign_bytes).map_err(|e| SignerError::Decode(e.into()))?;
		let sig = sig.normalize_s().unwrap_or(sig);
		let signature = secp256k1::Signature::try_from_bytes(&sig.to_bytes())
			.map_err(|e| SignerError::Decode(e.into()))?;
		Ok(signature)
	}

	/// Gets the public key in Sec1 format.
	async fn public_key(&self) -> Result<secp256k1::PublicKey, SignerError> {
		let res = self.client.get_public_key().key_id(&self.key_id).send().await.unwrap();
		//decode AWS public key
		let pk_ref: &[u8] = res.public_key().ok_or(SignerError::KeyNotFound)?.as_ref();
		let spki = spki::SubjectPublicKeyInfoRef::try_from(pk_ref)
			.map_err(|err| SignerError::PublicKey(Box::new(err)))?;
		let raw_bytes = spki.subject_public_key.raw_bytes();
		let public_key = secp256k1::PublicKey::try_from_bytes(&raw_bytes)
			.map_err(|e| SignerError::Decode(e.into()))?;
		Ok(public_key)
	}
}

impl AwsKmsSigner {
	pub async fn new(key_id: String) -> Self {
		let config = aws_config::load_from_env().await;
		let client = aws_sdk_kms::Client::new(&config);
		AwsKmsSigner { client, key_id }
	}

	/// Creates in AWS KMS matching the provided key id.
	pub async fn create_key(&self) -> Result<String, SignerError> {
		let res = self
			.client
			.create_key()
			.key_spec(KeySpec::EccSecgP256K1)
			.key_usage(KeyUsageType::SignVerify)
			.send()
			.await
			.map_err(|e| SignerError::Internal(e.to_string()))?;

		let key_id = res
			.key_metadata()
			.ok_or(SignerError::PublicKey("No key metadata available".into()))?
			.key_id()
			.to_string();

		Ok(key_id)
	}

	pub fn set_key_id(&mut self, key_id: String) {
		self.key_id = key_id;
	}
}
