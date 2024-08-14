use crate::backend::{PullOperations, PushOperations};
use crate::files::package::Package;

#[derive(Debug, Clone)]
pub struct Constant {
	pub package: Package,
}

#[async_trait::async_trait]
impl PullOperations for Constant {
	async fn pull(&self, _package: Package) -> Result<Package, anyhow::Error> {
		Ok(self.package.clone())
	}
}

#[async_trait::async_trait]
impl PushOperations for Constant {
	async fn push(&self, package: Package) -> Result<Package, anyhow::Error> {
		Ok(package)
	}
}
