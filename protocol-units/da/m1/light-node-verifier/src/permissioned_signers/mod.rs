use crate::{
	celestia::Verifier as CelestiaVerifier, signed::InKnownSignersVerifier, Error, Verified,
	VerifierOperations,
};
use celestia_rpc::Client;
use celestia_types::nmt::Namespace;
use celestia_types::Blob as CelestiaBlob;
use ecdsa::{
	elliptic_curve::{
		generic_array::ArrayLength,
		ops::Invert,
		point::PointCompression,
		sec1::{FromEncodedPoint, ModulusSize, ToEncodedPoint},
		subtle::CtOption,
		AffinePoint, CurveArithmetic, FieldBytesSize, PrimeCurve, Scalar,
	},
	hazmat::{DigestPrimitive, SignPrimitive, VerifyPrimitive},
	SignatureSize,
};
use m1_da_light_node_util::inner_blob::InnerBlob;
use std::collections::HashSet;
use std::sync::Arc;

#[derive(Clone)]
pub struct Verifier<C>
where
	C: PrimeCurve + CurveArithmetic + DigestPrimitive + PointCompression,
	Scalar<C>: Invert<Output = CtOption<Scalar<C>>> + SignPrimitive<C>,
	SignatureSize<C>: ArrayLength<u8>,
	AffinePoint<C>: FromEncodedPoint<C> + ToEncodedPoint<C> + VerifyPrimitive<C>,
	FieldBytesSize<C>: ModulusSize,
{
	pub celestia: CelestiaVerifier,
	pub known_signers: InKnownSignersVerifier<C>,
}

impl<C> Verifier<C>
where
	C: PrimeCurve + CurveArithmetic + DigestPrimitive + PointCompression,
	Scalar<C>: Invert<Output = CtOption<Scalar<C>>> + SignPrimitive<C>,
	SignatureSize<C>: ArrayLength<u8>,
	AffinePoint<C>: FromEncodedPoint<C> + ToEncodedPoint<C> + VerifyPrimitive<C>,
	FieldBytesSize<C>: ModulusSize,
{
	pub fn new(
		celestia_client: Arc<Client>,
		celestia_namespace: Namespace,
		known_signers_sec1_bytes: HashSet<String>,
	) -> Self {
		Self {
			celestia: CelestiaVerifier::new(celestia_client, celestia_namespace),
			known_signers: InKnownSignersVerifier::new(known_signers_sec1_bytes),
		}
	}
}

#[tonic::async_trait]
impl<C> VerifierOperations<CelestiaBlob, InnerBlob> for Verifier<C>
where
	C: PrimeCurve + CurveArithmetic + DigestPrimitive + PointCompression,
	Scalar<C>: Invert<Output = CtOption<Scalar<C>>> + SignPrimitive<C>,
	SignatureSize<C>: ArrayLength<u8>,
	AffinePoint<C>: FromEncodedPoint<C> + ToEncodedPoint<C> + VerifyPrimitive<C>,
	FieldBytesSize<C>: ModulusSize,
{
	async fn verify(&self, blob: CelestiaBlob, height: u64) -> Result<Verified<InnerBlob>, Error> {
		let verified_blob = self.celestia.verify(blob, height).await?;
		self.known_signers.verify(verified_blob.into_inner(), height).await
	}
}
