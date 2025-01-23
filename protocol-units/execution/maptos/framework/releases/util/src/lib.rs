use std::error;
use aptos_framework::ReleaseBundle;
#[derive(Debug, thiserror::Error)]
pub enum ReleaseBundleError {
	#[error("building release failed with error: {0}")]
	Build(#[source] Box<dyn error::Error + Send + Sync>),
}

pub trait Release {
	fn release(&self) -> Result<&'static ReleaseBundle, ReleaseBundleError>;
}
