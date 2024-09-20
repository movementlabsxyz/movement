use movement_types::application;
use std::path::PathBuf;
use syncador::backend::{archive, clear, glob, pipeline, s3, PullOperations, PushOperations};
use tokio::time::interval;
use tracing::info;

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
	) -> Result<(pipeline::push::Pipeline, pipeline::pull::Pipeline), anyhow::Error> {
		match self {
			Target::S3(bucket) => {
				let (s3_push, s3_pull) = s3::shared_bucket::create_random_with_application_id(
					bucket.clone(),
					application_id,
					root_dir.clone(),
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
) -> Result<impl std::future::Future<Output = Result<(), anyhow::Error>>, anyhow::Error> {
	info!("Running syncup with root {:?}, glob {}, and target {:?}", root_dir, glob, target);

	// create the pipelines for the target
	let (push_pipeline, pull_pipeline) =
		target.create_pipelines(root_dir.clone(), glob, application_id).await?;

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
			s3::shared_bucket::metadata::DEFAULT_SYNC_EPOCH_DURATION,
		));
		loop {
			info!("waiting for next push");
			interval.tick().await;
			let package = push_pipeline.push(syncador::Package::null()).await?;
			info!("Pushed package: {:?}", package);
		}
		Ok::<(), anyhow::Error>(())
	};

	Ok(upsync_task)
}
