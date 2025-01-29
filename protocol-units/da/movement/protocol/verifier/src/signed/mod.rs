use crate::{Error, Verified, VerifierOperations};
use movement_da_util::blob::ir::blob::DaBlob;
use movement_signer::{cryptography::Curve, Digester, Verify};
use std::collections::HashSet;

/// A verifier that checks the signature of the inner blob.
#[derive(Clone)]
pub struct Verifier<C>
where
	C: Curve + Verify<C>,
{
	pub _curve_marker: std::marker::PhantomData<C>,
}

impl<C> Verifier<C>
where
	C: Curve + Verify<C>,
{
	pub fn new() -> Self {
		Self { _curve_marker: std::marker::PhantomData }
	}
}

#[tonic::async_trait]
impl<C> VerifierOperations<DaBlob<C>, DaBlob<C>> for Verifier<C>
where
	C: Curve + Verify<C> + Digester<C> + Send + Sync + 'static,
{
	async fn verify(&self, blob: DaBlob<C>, _height: u64) -> Result<Verified<DaBlob<C>>, Error> {
		blob.verify_signature().map_err(|e| Error::Validation(e.to_string()))?;

		Ok(Verified::new(blob))
	}
}

/// Verifies that the signer of the inner blob is in the known signers set.
/// This is built around an inner signer because we should always check the signature first. That is, this composition prevents unsafe usage.
#[derive(Clone)]
pub struct InKnownSignersVerifier<C>
where
	C: Curve + Verify<C>,
{
	pub inner_verifier: Verifier<C>,
	/// The set of known signers in sec1 bytes hex format.
	pub known_signers_sec1_bytes_hex: HashSet<String>,
}

impl<C> InKnownSignersVerifier<C>
where
	C: Curve + Verify<C>,
{
	pub fn new<T>(known_signers_sec1_bytes_hex: T) -> Self
	where
		T: IntoIterator,
		T::Item: Into<String>,
	{
		Self {
			inner_verifier: Verifier::new(),
			known_signers_sec1_bytes_hex: known_signers_sec1_bytes_hex
				.into_iter()
				.map(Into::into)
				.collect(),
		}
	}
}

#[tonic::async_trait]
impl<C> VerifierOperations<DaBlob<C>, DaBlob<C>> for InKnownSignersVerifier<C>
where
	C: Curve + Verify<C> + Digester<C> + Send + Sync + 'static,
{
	async fn verify(&self, blob: DaBlob<C>, height: u64) -> Result<Verified<DaBlob<C>>, Error> {
		let da_blob = self.inner_verifier.verify(blob, height).await?;
		let signer = da_blob.inner().signer_hex();
		if !self.known_signers_sec1_bytes_hex.contains(&signer) {
			return Err(Error::Validation(
				format!("signer {} is not in the known signers set", signer).to_string(),
			));
		}

		Ok(da_blob)
	}
}

#[cfg(test)]
pub mod tests {
	// TODO: we need to recreate the signed verifier tests
}
