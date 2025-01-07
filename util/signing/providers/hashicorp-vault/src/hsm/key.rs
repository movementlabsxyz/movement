use crate::{cryptography::HashiCorpVaultCryptographySpec, hsm::HashiCorpVault};
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

impl<C> SignerBuilder<C, HashiCorpVault<C>> for Builder<C>
where
	C: Curve + HashiCorpVaultCryptographySpec + Sync,
{
	async fn build(&self, key: Key) -> Result<HashiCorpVault<C>, SignerBuilderError> {
		let mut hsm = HashiCorpVault::try_from_env()
			.map_err(|e| SignerBuilderError::Internal(e.to_string()))?;
		hsm.set_key_id(key.to_delimited_canonical_string("/"));
		Ok(hsm)
	}
}
