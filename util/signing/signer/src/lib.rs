pub mod cryptography;

/// Errors thrown by Signer
#[derive(Debug, thiserror::Error)]
pub enum SignerError {
	#[error("Error during signing : {0}")]
	Sign(String),
	#[error("Error during verification : {0}")]
	Verify(String),
	#[error("Error during public key retrieval : {0}")]
	PublicKey(String),
	#[error("Error can't decode provided hex data : {0}")]
	Hex(String),
	#[error("Signature not found.")]
	SignatureNotFound,
	#[error("public key not found.")]
	PublicKeyNotFound,
}

/// A collection of bytes.
#[derive(Debug, Clone)]
pub struct Bytes(pub Vec<u8>);

#[derive(Debug, Clone)]
pub struct KeyId {
	pub id: String,
	pub version: Option<String>,
}

/// A public key.
#[derive(Debug, Clone)]
pub struct PublicKey<C> {
	pub key_id: KeyId,
	pub curve: C,
	pub data: Bytes,
}

/// A signature.
#[derive(Debug, Clone)]
pub struct Signature {
	pub key_id: KeyId,
	pub data: Bytes,
}

#[async_trait::async_trait]
pub trait SignerOperations<C: cryptography::Curve> {
	/// Signs some bytes.
	async fn sign(&self, message: Bytes) -> Result<Signature, SignerError>;

	/// Gets the public key.
	async fn public_key(&self) -> Result<PublicKey<C>, SignerError>;
}

pub trait SignerOperationsSync<C: cryptography::Curve> {
	/// Signs some bytes.
	fn sign(&self, message: Bytes) -> Result<Signature, SignerError>;

	/// Gets the public key.
	fn public_key(&self) -> Result<PublicKey<C>, SignerError>;
}

#[async_trait::async_trait]
pub trait VerifierOperations<C: cryptography::Curve> {
	/// Verifies a signature.
	async fn verify(
		&self,
		message: Bytes,
		signature: Signature,
		public_key: PublicKey<C>,
	) -> Result<bool, SignerError>;
}

pub trait VerifierOperationsSync<C: cryptography::Curve> {
	/// Verifies a signature.
	fn verify(
		&self,
		message: Bytes,
		signature: Signature,
		public_key: PublicKey<C>,
	) -> Result<bool, SignerError>;
}

pub struct SignerConfig;

pub struct KeyManager {
	pub tobedefined: String,
}

impl KeyManager {
	pub fn build(config: SignerConfig) -> Self {
		Self { tobedefined: String::new() }
	}

	/// Get async signer for a key.
	pub fn get_async_signer<C: cryptography::Curve>(
		&self,
		key_id: KeyId,
	) -> Result<Box<dyn SignerOperations<C>>, SignerError> {
		todo!()
	}

	/// Get sync signer for a key.
	pub fn get_sync_signer<C: cryptography::Curve>(
		&self,
		key_id: KeyId,
	) -> Result<Box<dyn SignerOperationsSync<C>>, SignerError> {
		todo!()
	}

	/// Get async signer for a key.
	pub fn get_async_verifier<C: cryptography::Curve>(
		&self,
		key_id: KeyId,
	) -> Result<Box<dyn VerifierOperations<C>>, SignerError> {
		todo!()
	}

	/// Get sync signer for a key.
	pub fn get_sync_verifier<C: cryptography::Curve>(
		&self,
		key_id: KeyId,
	) -> Result<Box<dyn VerifierOperationsSync<C>>, SignerError> {
		todo!()
	}
}
