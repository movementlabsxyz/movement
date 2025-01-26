use aptos_framework::ReleaseBundle;
use maptos_framework_release_util::{Release, ReleaseBundleError};
use std::error;

pub enum KnownRelease {
	Elsa(aptos_framework_elsa_release::Elsa),
	Head(aptos_framework_head_release::Head),
}

impl Release for KnownRelease {
	fn release_bundle(&self) -> Result<ReleaseBundle, ReleaseBundleError> {
		match self {
			KnownRelease::Elsa(elsa) => elsa.release_bundle(),
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
			"elsa" => Ok(KnownRelease::Elsa(aptos_framework_elsa_release::Elsa::new())),
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
