use std::error;
use std::future::Future;

pub mod cryptography;

/// Errors thrown by Signer
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum SignerError {
	#[error("signing failed")]
	Sign(#[source] Box<dyn error::Error + Send + Sync>),
	#[error("failed to retrieve public key")]
	PublicKey(#[source] Box<dyn error::Error + Send + Sync>),
	#[error("failed to decode signer response")]
	Decode(#[source] Box<dyn error::Error + Send + Sync>),
	#[error("signing key not found")]
	KeyNotFound,
}

/// Asynchronous operations of a possibly remote signer.
///
/// The type parameter defines the elliptic curve used in the ECDSA signature algorithm.
pub trait Signer<C: cryptography::Curve> {
	/// Signs some bytes.
	fn sign(
		&self,
		message: &[u8],
	) -> impl Future<Output = Result<C::Signature, SignerError>> + Send;

	/// Fetches the public key that can be used for to verify signatures made by this signer.
	fn public_key(&self) -> impl Future<Output = Result<C::PublicKey, SignerError>> + Send;
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
