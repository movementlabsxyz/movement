pub mod cryptography;

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
	PublicKey(String),
	#[error("Error can't decode provided hex data : {0}")]
	Hex(String),
	#[error("Signature not found.")]
	SignatureNotFound,
	#[error("public key not found.")]
	PublicKeyNotFound,
}

#[async_trait::async_trait]
pub trait Signer {
	/// Signs some bytes.
	async fn sign(message: Bytes) -> Result<Signature, SignerError>;

	/// Gets the public key.
	async fn public_key(&self) -> Result<PublicKey, SignerError>;
}
