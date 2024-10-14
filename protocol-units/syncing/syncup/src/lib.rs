use movement_types::{actor, application};
use std::path::PathBuf;
use syncador::backend::{archive, clear, glob, pipeline, s3, PullOperations, PushOperations};
use tokio::time::interval;
use tracing::{info, warn};

pub trait SyncupOperations {
	/// Syncs up the files in the glob to the target.
	async fn syncup(
		self,
	) -> Result<impl std::future::Future<Output = Result<(), anyhow::Error>>, anyhow::Error>;

	/// Removes the syncing resources.
	async fn remove_syncup_resources(self) -> Result<(), anyhow::Error>;
}

pub trait Syncupable {
	/// Returns whether the current actor is the leader.
	fn try_leader(&self) -> Result<bool, anyhow::Error>;

	/// Returns the application id.
	fn try_application_id(&self) -> Result<application::Id, anyhow::Error>;

	/// Returns the syncer id.
	fn try_syncer_id(&self) -> Result<actor::Id, anyhow::Error>;

	/// Returns the root directory.
	fn try_root_dir(&self) -> Result<PathBuf, anyhow::Error>;

	/// Returns the glob pattern.
	fn try_glob(&self) -> Result<String, anyhow::Error>;

	/// Returns the target.
	fn try_target(&self) -> Result<Target, anyhow::Error>;
}

#[derive(Debug, Clone)]
pub enum Target {
	S3(String),
}

impl Target {
	pub async fn create_pipelines(
		&self,
		root_dir: PathBuf,
		glob: &str,
		application_id: application::Id,
		syncer_id: actor::Id,
	) -> Result<(pipeline::push::Pipeline, pipeline::pull::Pipeline), anyhow::Error> {
		info!("Creating pipelines for target {:?}", self);
		match self {
			Target::S3(bucket) => {
				let (s3_push, s3_pull) = s3::shared_bucket::create_with_load_from_env(
					bucket.clone(),
					root_dir.clone(),
					s3::shared_bucket::metadata::Metadata::default()
						.with_application_id(application_id)
						.with_syncer_id(syncer_id),
				)
				.await?;

				let push_pipe = pipeline::push::Pipeline::new(vec![
					Box::new(glob::file::FileGlob::try_new(glob, root_dir.clone())?),
					Box::new(archive::gzip::push::Push::new(root_dir.clone())),
					Box::new(s3_push),
				]);

				let pull_pipe = pipeline::pull::Pipeline::new(vec![
					Box::new(s3_pull),
					Box::new(clear::glob::pull::ClearGlob::try_new(glob, root_dir.clone())?),
					Box::new(archive::gzip::pull::Pull::new(root_dir.clone())),
				]);

				Ok((push_pipe, pull_pipe))
			}
		}
	}
}

/// Takes a glob pattern and a target, and syncs up the files in the glob to the target.
/// Returns two futures, one for the initial pull and one wrapped for the indefinite push.
pub async fn syncup(
	is_leader: bool,
	root_dir: PathBuf,
	glob: &str,
	target: Target,
	application_id: application::Id,
	syncer_id: actor::Id,
) -> Result<impl std::future::Future<Output = Result<(), anyhow::Error>>, anyhow::Error> {
	info!("Running syncup with root {:?}, glob {}, and target {:?}", root_dir, glob, target);

	// create the pipelines for the target
	let (push_pipeline, pull_pipeline) = target
		.create_pipelines(root_dir.clone(), glob, application_id, syncer_id)
		.await?;
	info!("Created pipelines");

	// run the pull pipeline once
	if !is_leader {
		info!("Running pull pipeline");
		let pull_package = pull_pipeline.pull(Some(syncador::Package::null())).await?;
		match pull_package {
			Some(package) => {
				info!("Pulled package: {:?}", package);
			}
			None => {
				info!("No package pulled");
			}
		}
	} else {
		info!("Skipping pull pipeline as leader");
	}

	// Create the upsync task using tokio::time::interval
	let upsync_task = async move {
		let mut interval = interval(std::time::Duration::from_millis(
			s3::shared_bucket::metadata::Metadata::DEFAULT_SYNC_EPOCH_DURATION,
		));
		loop {
			info!("waiting for next push");
			interval.tick().await;

			// push allow push to fail
			// ! This is a temporary solution to avoid competing forks in trusted environments.
			// ! This will be augmented with a more robust format in the future.
			if is_leader {
				info!("Running push pipeline");
				match push_pipeline.push(syncador::Package::null()).await {
					Ok(package) => {
						info!("Pushed package: {:?}", package);
					}
					Err(err) => {
						warn!("Error pushing package: {:?}", err);
					}
				}
			} else {
				info!("Non-leader upsyncing is disabled");
			}
		}
		Ok::<(), anyhow::Error>(())
	};

	Ok(upsync_task)
}

pub async fn remove_syncup_resources(target: Target) -> Result<(), anyhow::Error> {
	match target {
		Target::S3(bucket) => {
			s3::shared_bucket::destroy_with_load_from_env(bucket).await?;
		}
	}

	Ok(())
}

impl<T> SyncupOperations for T
where
	T: Syncupable,
{
	async fn syncup(
		self,
	) -> Result<impl std::future::Future<Output = Result<(), anyhow::Error>>, anyhow::Error> {
		let is_leader = self.try_leader()?;
		let root_dir = self.try_root_dir()?;
		let glob = self.try_glob()?;
		let target = self.try_target()?;
		let application_id = self.try_application_id()?;
		let syncer_id = self.try_syncer_id()?;

		syncup(is_leader, root_dir, &glob, target, application_id, syncer_id).await
	}

	async fn remove_syncup_resources(self) -> Result<(), anyhow::Error> {
		let target = self.try_target()?;
		remove_syncup_resources(target).await
	}
}
