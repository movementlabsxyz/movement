use std::path::PathBuf;
use syncador::backend::{archive, glob, pipeline, s3, PullOperations, PushOperations};
use tokio::time::interval;

#[derive(Debug, Clone)]
pub struct Notifier {
	pub sender: tokio::sync::mpsc::Sender<()>,
}

impl Notifier {
	pub fn new(size: usize) -> (Self, tokio::sync::mpsc::Receiver<()>) {
		let (sender, receiver) = tokio::sync::mpsc::channel(size);
		(Self { sender }, receiver)
	}
}

#[async_trait::async_trait]
impl PushOperations for Notifier {
	async fn push(&self, package: syncador::Package) -> Result<syncador::Package, anyhow::Error> {
		self.sender.send(()).await?;
		Ok(package)
	}
}

#[async_trait::async_trait]
impl PullOperations for Notifier {
	async fn pull(&self, package: syncador::Package) -> Result<syncador::Package, anyhow::Error> {
		self.sender.send(()).await?;
		Ok(package)
	}
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
		notifier: Notifier,
	) -> Result<(pipeline::push::Pipeline, pipeline::pull::Pipeline), anyhow::Error> {
		match self {
			Target::S3(bucket) => {
				let (s3_push, s3_pull) = s3::shared_bucket::create_random(bucket.clone()).await?;

				let push_pipe = pipeline::push::Pipeline::new(vec![
					Box::new(glob::file::FileGlob::try_new(glob, root_dir.clone())?),
					Box::new(archive::gzip::push::Push::new(root_dir.clone())),
					Box::new(s3_push),
					Box::new(notifier.clone()),
				]);

				let pull_pipe = pipeline::pull::Pipeline::new(vec![
					Box::new(s3_pull),
					Box::new(archive::gzip::pull::Pull::new(root_dir.clone())),
					Box::new(notifier.clone()),
				]);

				Ok((push_pipe, pull_pipe))
			}
		}
	}
}

/// Takes a glob pattern and a target, and syncs up the files in the glob to the target.
/// Returns two futures, one for the initial pull and one for the indefinite push.
pub async fn syncup(
	root_dir: PathBuf,
	glob: &str,
	target: Target,
	upsync_period: std::time::Duration,
) -> Result<(impl std::future::Future<Output = Result<(), anyhow::Error>>), anyhow::Error> {
	// create the pipelines for the target
	let (push_pipeline, pull_pipeline) =
		target.create_pipelines(root_dir.clone(), glob, Notifier::new(1).0).await?;

	// run the pull pipeline once
	pull_pipeline.pull(syncador::Package::null()).await?;

	// Create the upsync task using tokio::time::interval
	let upsync_task = async move {
		let mut interval = interval(upsync_period);
		loop {
			interval.tick().await;
			push_pipeline.push(syncador::Package::null()).await?;
		}
		Ok::<(), anyhow::Error>(())
	};

	Ok(upsync_task)
}
