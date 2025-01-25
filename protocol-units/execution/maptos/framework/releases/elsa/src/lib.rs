use aptos_framework::ReleaseBundle;
use maptos_framework_release_util::{CommitHash, Release, ReleaseBundleError};

pub static ELSA_COMMIT_HASH: &str = "9dfc8e7a3d622597dfd81cc4ba480a5377f87a41";
pub static ELSA_REPO: &str = "https://github.com/movementlabsxyz/aptos-core.git";
pub static ELSA_BYTECODE_VERSION: u32 = 6;

/// Builds a release for the Elsa framework.
/// This is a wrapper around the [CommitHash] builder.
///
/// Currently, we seem to be suffering from a bug related to: https://github.com/aptos-labs/aptos-core/issues/14913
pub struct Elsa(CommitHash);

impl Elsa {
	pub fn new() -> Self {
		Self(CommitHash::new(ELSA_REPO, ELSA_COMMIT_HASH, &ELSA_BYTECODE_VERSION))
	}
}

impl Release for Elsa {
	fn release(&self) -> Result<ReleaseBundle, ReleaseBundleError> {
		self.0.release()
	}
}
