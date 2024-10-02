use crate::backend::PushOperations;
use crate::files::package::Package;

pub struct Pipeline {
	pub backends: Vec<Box<dyn PushOperations + Send + Sync>>,
}

impl Pipeline {
	pub fn new(backends: Vec<Box<dyn PushOperations + Send + Sync>>) -> Self {
		Self { backends }
	}

	pub fn boxed(backends: Vec<Box<dyn PushOperations + Send + Sync>>) -> Box<Self> {
		Box::new(Self::new(backends))
	}
}

#[async_trait::async_trait]
impl PushOperations for Pipeline {
	async fn push(&self, package: Package) -> Result<Package, anyhow::Error> {
		let mut package = package;
		for backend in &self.backends {
			package = backend.push(package.clone()).await?;
		}
		Ok(package)
	}
}
