use crate::{Bytes, Hsm, PublicKey, Signature};
use anyhow::Context;
use google_cloud_kms::client::{Client, ClientConfig};
use google_cloud_kms::grpc::kms::v1::{
	AsymmetricSignRequest, CreateCryptoKeyRequest, CreateKeyRingRequest, CryptoKey, Digest,
	GetPublicKeyRequest,
};
use k256::ecdsa::{self, VerifyingKey};
use k256::pkcs8::DecodePublicKey;
use ring_compat::signature::Verifier;

pub struct GoogleKms {
	client: Client,
	project: String,
	location: String,
	key_ring: String,
	key_name: String,
	public_key: PublicKey,
}

impl GoogleKms {
	pub fn new(
		client: Client,
		project: String,
		location: String,
		key_ring: String,
		key_name: String,
		public_key: PublicKey,
	) -> Self {
		Self { client, project, location, key_ring, key_name, public_key }
	}

	/// Tries to create a new Google KMS HSM from the environment
	pub async fn try_from_env() -> Result<Self, anyhow::Error> {
		let project = std::env::var("GOOGLE_KMS_PROJECT").context("GOOGLE_KMS_PROJECT not set")?;
		let location =
			std::env::var("GOOGLE_KMS_LOCATION").context("GOOGLE_KMS_LOCATION not set")?;
		let key_ring =
			std::env::var("GOOGLE_KMS_KEY_RING").context("GOOGLE_KMS_KEY_RING not set")?;
		let key_name =
			std::env::var("GOOGLE_KMS_KEY_NAME").context("GOOGLE_KMS_KEY_NAME not set")?;
		let public_key = std::env::var("GOOGLE_KMS_PUBLIC_KEY").unwrap_or_default();

		let config = ClientConfig::default().with_auth().await?;
		let client = Client::new(config).await?;

		Ok(Self::new(
			client,
			project,
			location,
			key_ring,
			key_name,
			PublicKey(Bytes(public_key.as_bytes().to_vec())),
		))
	}

	/// Tries to create a new key matching the provided key name.
	pub async fn create_key_ring(self) -> Result<Self, anyhow::Error> {
		let request = CreateKeyRingRequest {
			parent: format!("projects/{}/locations/{}", self.project, self.location),
			key_ring_id: self.key_ring.clone(),
			key_ring: Default::default(),
		};

		self.client.create_key_ring(request, None).await?;
		Ok(self)
	}

	/// Tries to create a new key matching the provided key name.
	pub async fn create_key(self) -> Result<Self, anyhow::Error> {
		let request = CreateCryptoKeyRequest {
			parent: self.key_ring.clone(),
			crypto_key_id: self.key_name.clone(),
			crypto_key: Some(CryptoKey {
				purpose: 3, // Corresponds to ASYMETRIC_SIGN
				version_template: Some(Default::default()),
				..Default::default()
			}),
			skip_initial_version_creation: false,
		};

		self.client.create_crypto_key(request, None).await?;

		Ok(self)
	}

	/// Fills the public key from the key name
	pub async fn fill_with_public_key(mut self) -> Result<Self, anyhow::Error> {
		let request = GetPublicKeyRequest { name: self.key_name.clone() };

		let res = self.client.get_public_key(request, None).await?;

		self.public_key = PublicKey(Bytes(res.pem.as_bytes().to_vec()));

		Ok(self)
	}
}

#[async_trait::async_trait]
impl Hsm for GoogleKms {
	async fn sign(&self, message: Bytes) -> Result<(Bytes, PublicKey, Signature), anyhow::Error> {
		let digest = Digest {
			digest: Some(google_cloud_kms::grpc::kms::v1::digest::Digest::Sha256(
				message.clone().0,
			)),
			..Default::default()
		};

		let request = AsymmetricSignRequest {
			name: self.key_name.clone(),
			digest: Some(digest),
			..Default::default()
		};

		let response =
			self.client.asymmetric_sign(request, None).await.context("Failed to sign")?;

		let signature = Signature(Bytes(response.signature));

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

		// use the pkcs8 der to decode
		let k256_signature =
			ecdsa::Signature::from_der(&signature.0 .0).context("Failed to create signature")?;

		Ok(verifying_key.verify(message.0.as_slice(), &k256_signature).is_ok())
	}
}
