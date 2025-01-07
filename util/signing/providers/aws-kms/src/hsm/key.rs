use crate::{cryptography::AwsKmsCryptographySpec, hsm::AwsKms};
use movement_signer::{
	cryptography::Curve,
	key::{Key, SignerBuilder, SignerBuilderError},
};

pub struct Builder<C: Curve> {
	_cryptography_marker: std::marker::PhantomData<C>,
}

impl<C> Builder<C>
where
	C: Curve,
{
	pub fn new() -> Self {
		Self { _cryptography_marker: std::marker::PhantomData }
	}
}

impl<C> SignerBuilder<C, AwsKms<C>> for Builder<C>
where
	C: Curve + AwsKmsCryptographySpec + Sync,
{
	async fn build(&self, key: Key) -> Result<AwsKms<C>, SignerBuilderError> {
		let mut hsm = AwsKms::try_from_env()
			.await
			.map_err(|e| SignerBuilderError::Internal(e.to_string()))?;
		hsm.set_key_id(key.to_delimited_canonical_string("/"));
		Ok(hsm)
	}
}
