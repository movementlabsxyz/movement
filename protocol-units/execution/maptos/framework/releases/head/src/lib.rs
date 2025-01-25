use aptos_framework::ReleaseBundle;
use maptos_framework_release_util::{Release, ReleaseBundleError};

pub struct Head;

impl Head {
	pub fn new() -> Self {
		Self
	}
}

impl Release for Head {
	fn release(&self) -> Result<ReleaseBundle, ReleaseBundleError> {
		Ok(aptos_cached_packages::head_release_bundle().clone())
	}
}
