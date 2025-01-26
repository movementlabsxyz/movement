use anyhow::Context;
use std::path::PathBuf;

use aptos_framework::ReleaseBundle;
use maptos_framework_release_util::{CommitHash, Release, ReleaseBundleError};

pub static ELSA_COMMIT_HASH: &str = "9dfc8e7a3d622597dfd81cc4ba480a5377f87a41";
pub static ELSA_REPO: &str = "https://github.com/movementlabsxyz/aptos-core.git";
pub static ELSA_BYTECODE_VERSION: u32 = 6;
pub static CACHE_ELSA_FRAMEWORK_RELEASE: &str = "CACHE_ELSA_FRAMEWORK_RELEASE";
pub static ELSA_MRB: &str = "elsa.mrb";

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
	fn release_bundle(&self) -> Result<ReleaseBundle, ReleaseBundleError> {
		self.0.release_bundle()
	}
}

pub fn main() -> Result<(), anyhow::Error> {
	// Write to target/cache/elsa.mrb
	let target_cache_dir = PathBuf::from("target/mrb_cache");
	std::fs::create_dir_all(&target_cache_dir).context("failed to create cache directory")?;
	let path = target_cache_dir.join(ELSA_MRB);

	// if the release is already built and CACHE_ELSA_FRAMEWORK_RELEASE is set, skip building
	if std::env::var(CACHE_ELSA_FRAMEWORK_RELEASE).is_ok() && std::fs::metadata(&path).is_ok() {
		return Ok(());
	}

	// serialize the elsa release
	let elsa = Elsa::new();
	let release_bundle = elsa.release_bundle()?;
	let serialized_elsa =
		bcs::to_bytes(&release_bundle).context("failed to serialize elsa release")?;

	std::fs::write(&path, serialized_elsa).context("failed to write elsa release")?;

	Ok(())
}
