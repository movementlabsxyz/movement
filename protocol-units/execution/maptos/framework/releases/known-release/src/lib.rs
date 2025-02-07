use aptos_framework::ReleaseBundle;
use maptos_framework_release_util::{Release, ReleaseBundleError};
use std::error;

/// Known releases for the Aptos framework.
///
/// Before making a release, ensure that the release is cached. Lengthy build times can cause issues in e2e tests.
pub enum KnownRelease {
	Elsa(aptos_framework_elsa_release::cached::gas_upgrade::Elsa),
	BiarritzRc1(aptos_framework_biarritz_rc1_release::cached::full::feature_upgrade::BiarritzRc1),
	Head(aptos_framework_head_release::Head),
}

impl Release for KnownRelease {
	fn release_bundle(&self) -> Result<ReleaseBundle, ReleaseBundleError> {
		match self {
			KnownRelease::Elsa(elsa) => elsa.release_bundle(),
			KnownRelease::BiarritzRc1(biarritz_rc1) => biarritz_rc1.release_bundle(),
			KnownRelease::Head(head) => head.release_bundle(),
		}
	}
}

#[derive(Debug, thiserror::Error)]
pub enum KnownReleaseError {
	#[error("invalid identifier for known release: {0}")]
	InvalidIdentifier(#[source] Box<dyn error::Error + Send + Sync>),
}

/// Implement a method to create a new [KnownRelease] instance from a string.
impl KnownRelease {
	pub fn try_new(release: &str) -> Result<Self, KnownReleaseError> {
		match release {
			"elsa" => Ok(KnownRelease::Elsa(
				aptos_framework_elsa_release::cached::gas_upgrade::Elsa::new(),
			)),
			"biarritz-rc1" => Ok(KnownRelease::BiarritzRc1(
				aptos_framework_biarritz_rc1_release::cached::full::feature_upgrade::BiarritzRc1::new(),
			)),
			"head" => Ok(KnownRelease::Head(aptos_framework_head_release::Head::new())),
			_ => Err(KnownReleaseError::InvalidIdentifier(
				format!("unknown release string: {}", release).into(),
			)
			.into()),
		}
	}

	pub fn head() -> Self {
		KnownRelease::Head(aptos_framework_head_release::Head::new())
	}
}
