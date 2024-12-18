pub mod cryptography;

/// Version of a key.
/// Default mean the current key.
#[derive(Debug, Clone, Default)]
pub struct KeyVersion(pub String);

/// Id that identify a Key.
#[derive(Debug, Clone)]
pub struct KeyId(pub String);

/// Errors thrown by Signer
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
pub trait SignerOperations<C: cryptography::Curve> {
	/// Signs some bytes.
	async fn sign(&self, message: &[u8]) -> Result<C::Signature, SignerError>;

	/// Gets the public key.
	async fn public_key(&self) -> Result<C::PublicKey, SignerError>;
}

pub struct Signer<O, C>
where
	O: SignerOperations<C>,
	C: cryptography::Curve,
{
	operations: O,
	_curve_marker: std::marker::PhantomData<C>,
}

/// Signer wraps an implementation of [SignerOperations] and provides a simple API for signing and getting the public key.
impl<O, C> Signer<O, C>
where
	O: SignerOperations<C>,
	C: cryptography::Curve,
{
	pub fn new(operations: O) -> Self {
		Self { operations, _curve_marker: std::marker::PhantomData }
	}

	/// Signs some bytes.
	pub async fn sign(&self, message: &[u8]) -> Result<C::Signature, SignerError> {
		self.operations.sign(message).await
	}

	/// Gets the public key.
	pub async fn public_key(&self) -> Result<C::PublicKey, SignerError> {
		self.operations.public_key().await
	}
}

/// Errors thrown by the verifier.
#[derive(Debug, thiserror::Error)]
pub enum VerifierError {
	#[error("Error during verification : {0}")]
	Verify(String),
}

#[async_trait::async_trait]
pub trait VerifierOperations<C: cryptography::Curve> {
	/// Verifies a signature.
	async fn verify(
		&self,
		message: &[u8],
		signature: &C::Signature,
		public_key: &C::PublicKey,
	) -> Result<bool, VerifierError>;
}

pub struct Verifier<O, C>
where
	O: VerifierOperations<C>,
	C: cryptography::Curve,
{
	operations: O,
	_curve_marker: std::marker::PhantomData<C>,
}

/// Verifier wraps an implementation of [VerifierOperations] and provides a simple API for verifying signatures.
impl<O, C> Verifier<O, C>
where
	O: VerifierOperations<C>,
	C: cryptography::Curve,
{
	pub fn new(operations: O) -> Self {
		Self { operations, _curve_marker: std::marker::PhantomData }
	}

	/// Verifies a signature.
	pub async fn verify(
		&self,
		message: &[u8],
		signature: &C::Signature,
		public_key: &C::PublicKey,
	) -> Result<bool, VerifierError> {
		self.operations.verify(message, signature, public_key).await
	}
}
