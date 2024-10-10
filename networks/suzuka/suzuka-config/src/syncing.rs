use crate::Config as SuzukaConfig;
use anyhow::Context;
use dot_movement::DotMovement;
use godfig::env_or_none;
use movement_types::{actor, application};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use syncup::Syncupable;

/// The execution extension configuration.
/// This covers Suzuka configurations that do not configure the Maptos executor, but do configure the way it is used.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
	/// The number of times to retry a block if it fails to execute.
	#[serde(default = "default_movement_sync")]
	pub movement_sync: Option<String>,

	/// The application id.
	#[serde(default = "default_application_id")]
	pub application_id: application::Id,

	/// The syncer id.
	#[serde(default = "default_syncer_id")]
	pub syncer_id: actor::Id,

	/// The root directory.
	#[serde(default = "default_root_directory")]
	pub root_dir: PathBuf,
}

impl Default for Config {
	fn default() -> Self {
		Self {
			movement_sync: default_movement_sync(),
			application_id: default_application_id(),
			syncer_id: default_syncer_id(),
			root_dir: default_root_directory(),
		}
	}
}

pub fn default_movement_sync() -> Option<String> {
	std::env::var("MOVEMENT_SYNC").ok()
}

pub fn default_application_id() -> application::Id {
	application::Id::suzuka()
}

pub fn default_syncer_id() -> actor::Id {
	actor::Id::random()
}

pub fn default_root_directory() -> PathBuf {
	match DotMovement::try_from_env() {
		Ok(movement) => movement.get_path().to_path_buf(),
		Err(_) => PathBuf::from("./.movement"),
	}
}

pub struct MovementSync {
	pub is_leader: bool,
	pub bucket: String,
	pub glob: String,
}

impl TryFrom<String> for MovementSync {
	type Error = anyhow::Error;

	fn try_from(value: String) -> Result<Self, Self::Error> {
		// Split the string on "::", expect exactly two parts (leader/follower and sync-pattern)
		let (leader_follower_part, sync_pattern_part) = value.split_once("::").ok_or_else(|| anyhow!("MOVEMENT_SYNC environment variable must be in the format <leader|follower>::<sync-pattern>"))?;

		// Ensure there are no extra parts after splitting on "::"
		if leader_follower_split.next().is_some() {
			return Err(anyhow::anyhow!(
                "MOVEMENT_SYNC environment variable must be in the format <leader|follower>::<sync-pattern>"
            ));
		}

		// Validate leader/follower part
		let is_leader = match leader_follower_part {
			"leader" => true,
			"follower" => false,
			_ => {
				return Err(anyhow::anyhow!(
                "MOVEMENT_SYNC environment variable must start with either 'leader' or 'follower'"
            ))
			}
		};

		// Split sync pattern on "<=>", expect exactly two parts (bucket and glob)
		let mut bucket_arrow_glob = sync_pattern_part.split("<=>");
		let bucket = bucket_arrow_glob.next().context(
			"MOVEMENT_SYNC environment variable must be in the format <bucket><=> <glob>",
		)?;
		let glob = bucket_arrow_glob.next().context(
			"MOVEMENT_SYNC environment variable must be in the format <bucket><=> <glob>",
		)?;

		// Ensure there are no extra parts after splitting on "<=>"
		if bucket_arrow_glob.next().is_some() {
			return Err(anyhow::anyhow!(
				"MOVEMENT_SYNC environment variable must be in the format <bucket><=> <glob>"
			));
		}

		// Ensure both bucket and glob are non-empty
		if bucket.is_empty() || glob.is_empty() {
			return Err(anyhow::anyhow!(
				"MOVEMENT_SYNC environment variable must have non-empty <bucket> and <glob> values"
			));
		}

		// Return the parsed struct
		Ok(Self { is_leader, bucket: bucket.to_string(), glob: glob.to_string() })
	}
}

#[cfg(test)]
mod test_movement_sync {

	use super::MovementSync;

	#[test]
	fn test_try_from() {
		let movement_sync = MovementSync::try_from("leader::bucket<=>glob".to_string()).unwrap();
		assert_eq!(movement_sync.is_leader, true);
		assert_eq!(movement_sync.bucket, "bucket".to_string());
		assert_eq!(movement_sync.glob, "glob".to_string());

		let movement_sync = MovementSync::try_from("follower::bucket<=>glob".to_string()).unwrap();
		assert_eq!(movement_sync.is_leader, false);
		assert_eq!(movement_sync.bucket, "bucket".to_string());
		assert_eq!(movement_sync.glob, "glob".to_string());
	}

	#[test]
	fn test_try_from_error() {
		assert!(MovementSync::try_from("leader::bucket<=>".to_string()).is_err());
		assert!(MovementSync::try_from("leader::<=>glob".to_string()).is_err());
		assert!(MovementSync::try_from("leader::bucket".to_string()).is_err());
		assert!(MovementSync::try_from("leader::".to_string()).is_err());
		assert!(MovementSync::try_from("leader".to_string()).is_err());
	}

	#[test]
	fn test_multiple_matching_delimiters() {
		assert!(MovementSync::try_from("leader::bucket<=>glob<=>".to_string()).is_err());
		assert!(MovementSync::try_from("leader::<=>bucket<=>glob".to_string()).is_err());
		assert!(MovementSync::try_from("leader::bucket<=>glob<=>".to_string()).is_err());
		assert!(MovementSync::try_from("leader::bucket<=>glob<=>".to_string()).is_err());
	}
}

impl Config {
	/// Check if the args contain a movement sync.
	pub fn wants_movement_sync(&self) -> bool {
		self.movement_sync.is_some()
	}

	/// Get the DotMovement struct from the args.
	pub fn try_movement_sync(&self) -> Result<Option<MovementSync>, anyhow::Error> {
		Ok(self.movement_sync.clone().map(MovementSync::try_from).transpose()?)
	}
}

impl Syncupable for Config {
	fn try_application_id(&self) -> Result<application::Id, anyhow::Error> {
		Ok(self.application_id.clone())
	}

	fn try_glob(&self) -> Result<String, anyhow::Error> {
		let movement_sync =
			self.try_movement_sync()?.ok_or_else(|| anyhow::anyhow!("No movement sync"))?;
		Ok(movement_sync.glob)
	}

	fn try_leader(&self) -> Result<bool, anyhow::Error> {
		let movement_sync =
			self.try_movement_sync()?.ok_or_else(|| anyhow::anyhow!("No movement sync"))?;
		Ok(movement_sync.is_leader)
	}

	fn try_root_dir(&self) -> Result<PathBuf, anyhow::Error> {
		Ok(self.root_dir.clone())
	}

	fn try_syncer_id(&self) -> Result<actor::Id, anyhow::Error> {
		Ok(self.syncer_id.clone())
	}

	fn try_target(&self) -> Result<syncup::Target, anyhow::Error> {
		let movement_sync =
			self.try_movement_sync()?.ok_or_else(|| anyhow::anyhow!("No movement sync"))?;
		Ok(syncup::Target::S3(movement_sync.bucket))
	}
}

impl Syncupable for SuzukaConfig {
	fn try_application_id(&self) -> Result<application::Id, anyhow::Error> {
		self.syncing.try_application_id()
	}

	fn try_glob(&self) -> Result<String, anyhow::Error> {
		self.syncing.try_glob()
	}

	fn try_leader(&self) -> Result<bool, anyhow::Error> {
		self.syncing.try_leader()
	}

	fn try_root_dir(&self) -> Result<PathBuf, anyhow::Error> {
		self.syncing.try_root_dir()
	}

	fn try_syncer_id(&self) -> Result<actor::Id, anyhow::Error> {
		self.syncing.try_syncer_id()
	}

	fn try_target(&self) -> Result<syncup::Target, anyhow::Error> {
		self.syncing.try_target()
	}
}
