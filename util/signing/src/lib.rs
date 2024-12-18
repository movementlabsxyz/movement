/// A collection of bytes.
#[derive(Debug, Clone)]
pub struct Bytes(pub Vec<u8>);

/// A signature.
#[derive(Debug, Clone)]
pub struct Signature(pub Bytes);

/// A public key.
#[derive(Debug, Clone)]
pub struct PublicKey(pub Bytes);

/// Version of a key.
/// Default mean the current key.
#[derive(Debug, Clone, Default)]
pub struct KeyVersion(pub String);

/// Id that identify a Key.
#[derive(Debug, Clone)]
pub struct KeyId(pub String);

/// Errors thrown by SigningService.
#[derive(Debug, thiserror::Error)]
pub enum SignerError {
	#[error("Error during signing : {0}")]
	Sign(String),
	#[error("Error during public key retrieval : {0}")]
	GetPublicKey(String),
	#[error("Error can't decode provided hex data : {0}")]
	Hex(String),
	#[error("Signature not found.")]
	SignatureNotFound,
	#[error("public key not found.")]
	PublicKeyNotFound,
}

pub struct SigningService;

impl SigningService {
	/// Create the service with environment variable.
	pub fn try_from_env() -> Result<Self, SignerError> {
		todo!()
	}

	/// Sign the provided message with the current key identified with the keyId.
	/// Return the Signature and the version of the key used to sign.
	pub async fn sign(
		&self,
		message: Bytes,
		key: KeyId,
	) -> Result<(KeyVersion, Signature), SignerError> {
		todo!();
	}

	/// Get the public key associated with the specified key and version.
	pub async fn get_public_key(
		&self,
		key: KeyId,
		version: KeyVersion,
	) -> Result<PublicKey, SignerError> {
		todo!();
	}
}
