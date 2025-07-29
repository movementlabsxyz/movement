use crate::backend::s3::bucket_connection::BucketConnection;
use movement_types::{actor, application};
use std::collections::HashSet;
use std::time;

#[derive(Debug, Clone)]
pub struct Metadata {
	pub application_id: application::Id,
	pub syncer_id: actor::Id,
	pub sync_epoch_duration: u64,
	pub retain_epochs_count: u64,
}

impl Metadata {
	pub const DEFAULT_RETAIN_EPOCHS_COUNT: u64 = 16;
	pub const DEFAULT_SYNC_EPOCH_DURATION: u64 = 1000 * 60;

	pub fn new(
		application_id: application::Id,
		syncer_id: actor::Id,
		sync_epoch_duration: u64,
		retain_epochs_count: u64,
	) -> Self {
		Self { application_id, syncer_id, sync_epoch_duration, retain_epochs_count }
	}

	/// Set the application id for the metadata
	pub fn with_application_id(self, application_id: application::Id) -> Self {
		Self { application_id, ..self }
	}

	/// Set the syncer id for the metadata
	pub fn with_syncer_id(self, syncer_id: actor::Id) -> Self {
		Self { syncer_id, ..self }
	}

	/// Set the sync epoch duration for the metadata
	pub fn random() -> Self {
		Self {
			application_id: application::Id::random(),
			syncer_id: actor::Id::random(),
			sync_epoch_duration: Self::DEFAULT_SYNC_EPOCH_DURATION,
			retain_epochs_count: Self::DEFAULT_RETAIN_EPOCHS_COUNT,
		}
	}

	pub fn default() -> Self {
		Self {
			application_id: application::Id::default(),
			syncer_id: actor::Id::default(),
			sync_epoch_duration: Self::DEFAULT_SYNC_EPOCH_DURATION,
			retain_epochs_count: Self::DEFAULT_RETAIN_EPOCHS_COUNT,
		}
	}

	pub(crate) fn get_sync_epoch(&self) -> Result<u64, anyhow::Error> {
		let now = time::SystemTime::now().duration_since(time::UNIX_EPOCH)?.as_millis() as u64;

		// sync epoch should be milliseconds floored by the epoch duration
		Ok((now / self.sync_epoch_duration) * self.sync_epoch_duration)
	}

	pub(crate) fn application_prefix(&self) -> Result<String, anyhow::Error> {
		Ok(self.application_id.to_string())
	}

	pub(crate) fn syncer_prefix(&self) -> Result<String, anyhow::Error> {
		Ok(format!("{}/{}", self.application_prefix()?, self.syncer_id.to_string()))
	}

	pub(crate) fn syncer_epoch_prefix(&self) -> Result<String, anyhow::Error> {
		Ok(format!("{}/{}", self.syncer_prefix()?, self.get_sync_epoch()?))
	}

	pub(crate) async fn list_all_application_file_paths_for(
		&self,
		bucket_connection: &BucketConnection,
	) -> Result<HashSet<String>, anyhow::Error> {
		let prefix = self.application_id.to_string();
		let mut continuation_token = None;
		let mut file_paths = HashSet::new();
		loop {
			let list_objects_output = bucket_connection
				.client
				.list_objects_v2()
				.bucket(bucket_connection.bucket.clone())
				.prefix(&prefix)
				.set_continuation_token(continuation_token)
				.send()
				.await?;
			if let Some(contents) = list_objects_output.contents {
				for object in contents {
					if let Some(key) = object.key {
						file_paths.insert(key);
					}
				}
			}
			if let Some(token) = list_objects_output.next_continuation_token {
				continuation_token = Some(token);
			} else {
				break;
			}
		}
		Ok(file_paths)
	}

	pub(crate) async fn list_all_application_syncer_epochs(
		&self,
		connection: &BucketConnection,
	) -> Result<HashSet<u64>, anyhow::Error> {
		// list all of the objects at the first level of depth below application/syncer
		let prefix = self.syncer_prefix()?;

		let mut continuation_token = None;
		let mut sync_epochs = HashSet::new();
		loop {
			let list_objects_output = connection
				.client
				.list_objects_v2()
				.bucket(connection.bucket.clone())
				.prefix(&prefix)
				.set_continuation_token(continuation_token)
				.send()
				.await?;
			if let Some(contents) = list_objects_output.contents {
				for object in contents {
					if let Some(key) = object.key {
						let parts: Vec<&str> = key.split('/').into_iter().collect();
						if parts.len() > 1 {
							if let Ok(sync_epoch) = parts[2].parse::<u64>() {
								sync_epochs.insert(sync_epoch);
							}
						}
					}
				}
			}
			if let Some(token) = list_objects_output.next_continuation_token {
				continuation_token = Some(token);
			} else {
				break;
			}
		}

		Ok(sync_epochs)
	}
}
