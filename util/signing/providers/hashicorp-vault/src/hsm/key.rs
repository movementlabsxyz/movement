use crate::{cryptography::HashiCorpVaultCryptographySpec, hsm::HashiCorpVault};
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

impl<C> SignerBuilder<C, HashiCorpVault<C>> for Builder<C>
where
	C: Curve + HashiCorpVaultCryptographySpec + Send + Sync,
{
	async fn build(&self, key: Key) -> Result<HashiCorpVault<C>, SignerBuilderError> {
		let mut hsm = HashiCorpVault::try_from_env()
			.map_err(|e| SignerBuilderError::Internal(e.to_string()))?;
		hsm.set_key_id(key.to_delimited_canonical_string("/"));
		if self.create_key {
			hsm = hsm
				.create_key()
				.await
				.map_err(|e| SignerBuilderError::Internal(e.to_string()))?;
		}
		Ok(hsm)
	}
}
