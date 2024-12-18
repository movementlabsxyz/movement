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

pub struct SigningService;

impl SigningService {
	/// Create the service with environment variable.
	pub fn try_from_env() -> Result<Self, anyhow::Error> {
		todo!()
	}

	/// Sign the provided message with the current key identified with the keyId.
	/// Return the Signature and the version of the key used to sign.
	async fn sign(
		&self,
		message: Bytes,
		key: KeyId,
	) -> Result<(KeyVersion, Signature), anyhow::Error> {
		todo!();
	}

	/// Get the public key associated with the specified key and version.
	async fn get_public_key(
		&self,
		key: KeyId,
		version: KeyVersion,
	) -> Result<PublicKey, anyhow::Error> {
		todo!();
	}
}
