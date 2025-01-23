use aptos_framework::ReleaseBundle;
use aptos_release_builder::components::framework::{
	generate_upgrade_proposals_with_repo, FrameworkReleaseConfig,
};
use std::error;
#[derive(Debug, thiserror::Error)]
pub enum ReleaseBundleError {
	#[error("building release failed with error: {0}")]
	Build(#[source] Box<dyn error::Error + Send + Sync>),
}

pub trait Release {
	fn release(&self) -> Result<&'static ReleaseBundle, ReleaseBundleError>;
}

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
	fn release(&self) -> Result<&'static ReleaseBundle, ReleaseBundleError> {
		let (config, repo) = self.framework_release_config();

		let upgrade_proposals = generate_upgrade_proposals_with_repo(&config, true, vec![], repo)
			.map_err(|e| ReleaseBundleError::Build(e.into()))?;

		Ok(aptos_cached_packages::commit_hash_release_bundle(self.0))
	}
}
