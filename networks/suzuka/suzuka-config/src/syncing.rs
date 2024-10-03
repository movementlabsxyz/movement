use crate::Config as SuzukaConfig;
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
	pub root_dir: String,

	/// The glob pattern.
	
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
	DotMovement::try_from_env().unwrap_or_else(|_| "./.movement".into())
}

pub struct MovementSync {
	pub is_leader: bool,
	pub bucket: String,
	pub glob: String,
}

impl TryFrom<String> for MovementSync {
	type Error = anyhow::Error;

	fn try_from(value: String) -> Result<Self, Self::Error> {
		let mut leader_follower_split = sync_str.split("::");
		let is_leader = leader_follower_split.next().context(
			"MOVEMENT_SYNC environment variable must be in the format <leader|follower>::<sync-pattern>",
		)? == "leader";

		let mut bucket_arrow_glob = leader_follower_split.next().context(
			"MOVEMENT_SYNC environment variable must be in the format <leader|follower>::<sync-pattern>",
		)?.split("<=>");

		let bucket = bucket_arrow_glob.next().context(
			"MOVEMENT_SYNC environment variable must be in the format <bucket>,<glob>",
		)?;
		let glob = bucket_arrow_glob.next().context(
			"MOVEMENT_SYNC environment variable must be in the format <bucket>,<glob>",
		)?;

		Ok(Self { is_leader, bucket, glob })
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


impl Syncupable for SuzukaConfig {
	fn try_application_id(&self) -> Result<application::Id, anyhow::Error> {
		Ok(self.application_id.clone())
	}

	fn try_glob(&self) -> Result<String, anyhow::Error> {
		let movement_sync = self.try_movement_sync()?;
		Ok(movement_sync.glob)
	}

	fn try_leader(&self) -> Result<bool, anyhow::Error> {
		let movement_sync = self.try_movement_sync()?;
		Ok(movement_sync.is_leader)
	}

	fn try_root_dir(&self) -> Result<PathBuf, anyhow::Error> {
		Ok(self.root_dir.clone())
	}

	fn try_syncer_id(&self) -> Result<actor::Id, anyhow::Error> {
		Ok(self.syncer_id.clone())
	}

	fn try_target(&self) -> Result<syncup::Target, anyhow::Error> {
		let movement_sync = self.try_movement_sync()?;
		Ok(syncup::Target::S3(movement_sync.bucket))
	}
}
