use crate::{
	celestia::Verifier as CelestiaVerifier, signed::InKnownSignersVerifier, Error, Verified,
	VerifierOperations,
};
use celestia_rpc::Client;
use celestia_types::nmt::Namespace;
use celestia_types::Blob as CelestiaBlob;
use movement_celestia_da_util::ir_blob::IntermediateBlobRepresentation;
use movement_signer::{cryptography::Curve, Verify};
use std::sync::Arc;

/// A verifier of Celestia blobs for permissioned signers
#[derive(Clone)]
pub struct Verifier<C>
where
	C: Curve + Verify<C>,
{
	/// The Celestia veifier
	pub celestia: CelestiaVerifier,
	/// The verifier for known signers
	pub known_signers: InKnownSignersVerifier<C>,
}

impl<C> Verifier<C>
where
	C: Curve + Verify<C>,
{
	pub fn new<T>(
		celestia_client: Arc<Client>,
		celestia_namespace: Namespace,
		known_signers_sec1_bytes: T,
	) -> Self
	where
		T: IntoIterator,
		T::Item: Into<String>,
	{
		Self {
			celestia: CelestiaVerifier::new(celestia_client, celestia_namespace),
			known_signers: InKnownSignersVerifier::new(known_signers_sec1_bytes),
		}
	}
}

#[tonic::async_trait]
impl<C> VerifierOperations<CelestiaBlob, IntermediateBlobRepresentation> for Verifier<C>
where
	C: Curve + Verify<C> + Send + Sync,
{
	async fn verify(
		&self,
		blob: CelestiaBlob,
		height: u64,
	) -> Result<Verified<IntermediateBlobRepresentation>, Error> {
		let verified_blob = self.celestia.verify(blob, height).await?;
		self.known_signers.verify(verified_blob.into_inner(), height).await
	}
}
