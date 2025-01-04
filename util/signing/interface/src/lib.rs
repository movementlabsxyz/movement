use std::error;
use std::future::Future;
use std::marker::PhantomData;

pub mod cryptography;
pub mod manager;

/// Errors thrown by Signer
#[derive(Debug, thiserror::Error)]
pub enum SignerError {
	#[error("signing failed")]
	Sign(#[source] Box<dyn error::Error + Send + Sync>),
	#[error("failed to retrieve public key")]
	PublicKey(#[source] Box<dyn error::Error + Send + Sync>),
	#[error("failed to decode signer response")]
	Decode(#[source] Box<dyn error::Error + Send + Sync>),
	#[error("signing key not found")]
	KeyNotFound,
	#[error("failed to sign")]
	Internal(String),
}

/// Asynchronous operations of a possibly remote signing service.
///
/// The type parameter defines the elliptic curve used in the ECDSA signature algorithm.
pub trait Signing<C: cryptography::Curve> {
	/// Signs some bytes.
	fn sign(
		&self,
		message: &[u8],
	) -> impl Future<Output = Result<C::Signature, SignerError>> + Send;

	/// Fetches the public key that can be used for to verify signatures made by this signer.
	fn public_key(&self) -> impl Future<Output = Result<C::PublicKey, SignerError>> + Send;
}

/// A convenience struct to bind a signing service with the specific elliptic curve type,
/// so as to provide an ergonomic signing API without the need to fully qualify the curve parameter
/// in method calls.
#[derive(Debug, Clone, Copy)]
pub struct Signer<O, C> {
	provider: O,
	_phantom_curve: PhantomData<C>,
}

impl<O, C> Signer<O, C> {
	/// Binds the signing provider with the specific curve selection.
	pub fn new(provider: O, curve: C) -> Self {
		let _ = curve;
		Self { provider, _phantom_curve: PhantomData }
	}

	/// Unwraps the inner signing provider object.
	pub fn into_inner(self) -> O {
		self.provider
	}
}

impl<O, C> Signer<O, C>
where
	O: Signing<C>,
	C: cryptography::Curve,
{
	/// Signs some bytes.
	pub async fn sign(&self, message: &[u8]) -> Result<C::Signature, SignerError> {
		self.provider.sign(message).await
	}

	/// Fetches the public key that can be used for to verify signatures made by this signer.
	pub async fn public_key(&self) -> Result<C::PublicKey, SignerError> {
		self.provider.public_key().await
	}
}

/// Errors thrown by the verifier.
#[derive(Debug, thiserror::Error)]
#[error("failed to verify signature")]
pub struct VerifyError(#[source] Box<dyn error::Error + Send + Sync>);

/// A signature verifier.
///
/// The type parameter defines the elliptic curve used in the ECDSA signature algorithm.
/// In contrast with implementations of [`Signer`], the verifier does not need to be
/// remote or asynchronous, as all data that it uses to verify the signature is not secret
/// and immediately available.
pub trait Verify<C: cryptography::Curve> {
	/// Verifies a signature.
	fn verify(
		&self,
		message: &[u8],
		signature: &C::Signature,
		public_key: &C::PublicKey,
	) -> Result<bool, VerifyError>;
}
