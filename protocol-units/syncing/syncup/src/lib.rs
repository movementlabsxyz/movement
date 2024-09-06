use std::path::PathBuf;
use syncador::backend::{archive, glob, pipeline, s3, PullOperations, PushOperations};
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
	) -> Result<(pipeline::push::Pipeline, pipeline::pull::Pipeline), anyhow::Error> {
		match self {
			Target::S3(bucket) => {
				let (s3_push, s3_pull) = s3::shared_bucket::create_random(bucket.clone()).await?;

				let push_pipe = pipeline::push::Pipeline::new(vec![
					Box::new(glob::file::FileGlob::try_new(glob, root_dir.clone())?),
					Box::new(archive::gzip::push::Push::new(root_dir.clone())),
					Box::new(s3_push),
				]);

				let pull_pipe = pipeline::pull::Pipeline::new(vec![
					Box::new(s3_pull),
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
	root_dir: PathBuf,
	glob: &str,
	target: Target,
) -> Result<impl std::future::Future<Output = Result<(), anyhow::Error>>, anyhow::Error> {
	// create the pipelines for the target
	let (push_pipeline, pull_pipeline) = target.create_pipelines(root_dir.clone(), glob).await?;

	// run the pull pipeline once
	let pull_package = pull_pipeline.pull(Some(syncador::Package::null())).await?;

	match pull_package {
		Some(package) => {
			info!("Pulled package: {:?}", package);
		}
		None => {
			info!("No package pulled");
		}
	}

	// Create the upsync task using tokio::time::interval
	let upsync_task = async move {
		let mut interval = interval(std::time::Duration::from_millis(
			s3::shared_bucket::metadata::DEFAULT_SYNC_EPOCH_DURATION,
		));
		loop {
			interval.tick().await;
			push_pipeline.push(syncador::Package::null()).await?;
		}
		Ok::<(), anyhow::Error>(())
	};

	Ok(upsync_task)
}
