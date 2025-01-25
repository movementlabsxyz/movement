use aptos_framework::{BuiltPackage, ReleaseBundle, ReleasePackage};
use aptos_release_builder::components::framework::{
	generate_upgrade_proposals_release_packages_with_repo, FrameworkReleaseConfig,
};
use std::sync::Arc;

use std::error;
#[derive(Debug, thiserror::Error)]
pub enum ReleaseBundleError {
	#[error("building release failed with error: {0}")]
	Build(#[source] Box<dyn error::Error + Send + Sync>),
}

pub trait Release {
	fn release(&self) -> Result<ReleaseBundle, ReleaseBundleError>;
}

/// To form a commit hash porposer, at the lowest level we use [generate_upgrade_proposals_with_repo] function to generate the scripts.
/// We then write these scripts out to a proposal directory in line with the implementation here: https://github.com/movementlabsxyz/aptos-core/blob/ac9de113a4afec6a26fe587bb92c982532f09d3a/aptos-move/aptos-release-builder/src/components/mod.rs#L563
/// We then need to compile the code to form [ReleasePackage]s which are then used to form [ReleaseBundle]s.
/// To do this, we need to form a [BuiltPackage] from the scripts I BELIEVE.
pub struct CommitHash {
	pub repo: &'static str,
	pub commit_hash: &'static str,
	pub bytecode_version: &'static u32,
}

impl CommitHash {
	pub fn new(
		repo: &'static str,
		commit_hash: &'static str,
		bytecode_version: &'static u32,
	) -> Self {
		Self { repo, commit_hash, bytecode_version }
	}

	pub fn framework_release_config(&self) -> (FrameworkReleaseConfig, &'static str) {
		let config = FrameworkReleaseConfig {
			bytecode_version: *self.bytecode_version,
			git_hash: Some(self.commit_hash.to_string()),
		};
		(config, self.repo)
	}
}

impl Release for CommitHash {
	fn release(&self) -> Result<ReleaseBundle, ReleaseBundleError> {
		let (config, repo) = self.framework_release_config();

		let (_commit_info, releases) =
			generate_upgrade_proposals_release_packages_with_repo(&config, true, vec![], repo)
				.map_err(|e| ReleaseBundleError::Build(e.into()))?;
		let release_packages = releases
			.into_iter()
			.map(|(_account, release_package, _move_script_path, _script_name)| release_package)
			.collect();

		let release_bundle = ReleaseBundle::new(release_packages, vec![]);

		Ok(release_bundle)
	}
}

/// A dynamic wrapper around a [Release] implementation.
pub struct CommonRelease(pub Arc<dyn Release>);

impl CommonRelease {
	pub fn new(release: Arc<dyn Release>) -> Self {
		Self(release)
	}
}

impl Release for CommonRelease {
	fn release(&self) -> Result<ReleaseBundle, ReleaseBundleError> {
		self.0.release()
	}
}
