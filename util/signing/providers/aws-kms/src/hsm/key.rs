use crate::{cryptography::AwsKmsCryptographySpec, hsm::AwsKms};
use movement_signer::{
	cryptography::Curve,
	key::{Key, SignerBuilder, SignerBuilderError},
};

pub struct Builder<C: Curve> {
	create_key: bool,
	_cryptography_marker: std::marker::PhantomData<C>,
}

impl<C> Builder<C>
where
	C: Curve,
{
	pub fn new() -> Self {
		Self { create_key: false, _cryptography_marker: std::marker::PhantomData }
	}

	pub fn create_key(mut self, create_key: bool) -> Self {
		self.create_key = create_key;
		self
	}
}

impl<C> SignerBuilder<C, AwsKms<C>> for Builder<C>
where
	C: Curve + AwsKmsCryptographySpec + Send + Sync,
	AwsKms<C>: movement_signer::Signing<C>,
{
	async fn build(&self, key: Key) -> Result<AwsKms<C>, SignerBuilderError> {
		let mut hsm = AwsKms::try_from_env()
			.await
			.map_err(|e| SignerBuilderError::Internal(e.to_string()))?;
		// AWS Key id is defined by the key name.
		hsm.set_key_id(key.key_name().to_string());
		if self.create_key {
			hsm = hsm
				.create_key()
				.await
				.map_err(|e| SignerBuilderError::Internal(e.to_string()))?;
		}
		Ok(hsm)
	}
}
