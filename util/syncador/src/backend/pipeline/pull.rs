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
	async fn pull(&self, package: Package) -> Result<Package, anyhow::Error> {
		let mut package = package;
		for backend in &self.backends {
			package = backend.pull(package.clone()).await?;
		}
		Ok(package)
	}
}
