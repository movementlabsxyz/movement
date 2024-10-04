use tracing::info;

use crate::backend::PullOperations;
use crate::files::package::Package;

pub struct Pipeline {
	pub backends: Vec<Box<dyn PullOperations + Send + Sync>>,
}

impl Pipeline {
	pub fn new(backends: Vec<Box<dyn PullOperations + Send + Sync>>) -> Self {
		Self { backends }
	}

	pub fn boxed(backends: Vec<Box<dyn PullOperations + Send + Sync>>) -> Box<Self> {
		Box::new(Self::new(backends))
	}
}

#[async_trait::async_trait]
impl PullOperations for Pipeline {
	async fn pull(&self, package: Option<Package>) -> Result<Option<Package>, anyhow::Error> {
		let mut package = package;
		for backend in &self.backends {
			info!("Pulling from backend");
			package = backend.pull(package.clone()).await?;
		}
		Ok(package)
	}
}
